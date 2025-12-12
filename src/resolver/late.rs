use crate::error::CompilerError;
use crate::parser::ast::{AstNode, BlockExpr, Expr, Ident, Item, LetStmt, Path, PathSegment, Pattern};
use crate::resolver::defs::DefKind;
use crate::resolver::modules::{Binding, ModuleId, ModuleKind};
use crate::resolver::ribs::{PrimTy, Res, Rib, RibKind};
use crate::resolver::{visitor, ResolverError};
use crate::{visit_opt, Resolver};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PatternSource {
    Match,  // constructor or binding
    Normal, // binding
}

pub struct LateResolver<'a, 'r> {
    r: &'a mut Resolver<'r>,
    ribs: Vec<Rib>,
    parent: ModuleId,
}

impl<'a, 'r> LateResolver<'a, 'r> {
    pub fn new(r: &'a mut Resolver<'r>, root: ModuleId) -> Self {
        Self {
            r,
            ribs: vec![Rib::item()],
            parent: root,
        }
    }

    fn innermost_rib(&mut self) -> &mut Rib {
        self.ribs.last_mut().unwrap()
    }

    fn push_rib(&mut self, kind: RibKind) {
        self.ribs.push(Rib::new(kind))
    }
    fn pop_rib(&mut self) {
        self.ribs.pop();
    }

    fn with_rib<F>(&mut self, kind: RibKind, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        self.push_rib(kind);
        f(self);
        self.pop_rib();
    }

    fn resolve_pattern(&mut self, pattern: &AstNode<Pattern>, source: PatternSource) {
        match &pattern.node {
            Pattern::Or(patterns) => {
                // TODO: All alternatives must bind the same names
                for pat in patterns {
                    self.resolve_pattern(pat, source);
                }
            }
            Pattern::Path(path) => {
                if path.node.segments.len() == 1 {
                    let segment = &path.node.segments[0];
                    let name = &segment.node.ident;

                    if matches!(source, PatternSource::Match) {
                        if let Some(res) = self.lookup_value(&name.node) {
                            match res {
                                Res::Local(_) | Res::PrimTy(_) => todo!("probably insert as binding for PrimTy"),
                                Res::Def(def_id) => {
                                    self.r.defs.insert_ast_id(path.ast_id, def_id);
                                    return;
                                }
                            }
                        }
                    }
                    self.define_binding(&name, pattern);
                } else {
                    self.resolve_path(path);
                }
            }
            _ => visitor::walk_pattern(self, pattern),
        }
    }

    fn lookup_ribs(&self, ident: &Ident) -> Option<Res> {
        for rib in self.ribs.iter().rev() {
            if let Some(ast_id) = rib.get(ident) {
                return Some(Res::Local(ast_id));
            }
        }
        None
    }

    fn lookup_modules(&self, ident: &Ident, module_id: ModuleId) -> Option<Res> {
        let module = self.r.module_arena.get(module_id);
        match module.get(ident) {
            None => self.lookup_modules(ident, module.parent()?),
            Some(binding) => match binding {
                Binding::Item(def_id) => Some(Res::Def(*def_id)),
                _ => unreachable!(),
            },
        }
    }

    fn lookup_value(&self, ident: &Ident) -> Option<Res> {
        self.lookup_ribs(ident)
            .or_else(|| self.lookup_modules(ident, self.parent))
            .or_else(|| self.lookup_prim_ty(ident))
    }

    fn lookup_prim_ty(&self, ident: &Ident) -> Option<Res> {
        PrimTy::from_name(&ident.name).map(Res::PrimTy)
    }

    fn define_binding(&mut self, ident: &AstNode<Ident>, pattern: &AstNode<Pattern>) {
        if self.innermost_rib().get(&ident.node).is_some() {
            self.r
                .session
                .push_error(CompilerError::ResolverError(ResolverError::DuplicateDefinition {
                    src: self.r.session.get_named_source(),
                    span: ident.span,
                    name: ident.node.name.clone(),
                }));
        }
        self.innermost_rib().insert(ident.node.clone(), pattern.ast_id);
    }

    fn resolve_path(&mut self, path: &AstNode<Path>) {
        if path.node.segments.is_empty() {
            return;
        }

        let segments = &path.node.segments;
        let (mut current_module, segment_start) = self.resolve_first_segment(segments);

        if segments.len() == 1 && segment_start == 0 {
            let first_ident = &segments[0].node.ident.node;
            if self.lookup_value(first_ident).is_none() {
                self.report_unresolved_path(path);
            }
            return;
        }

        for (i, segment) in segments.iter().enumerate().skip(segment_start) {
            let ident = &segment.node.ident.node;

            let binding = self.resolve_ident_in_module(current_module, ident);

            match binding {
                Some(binding) => match binding {
                    Binding::Module(module_id) => {
                        current_module = module_id;
                    }
                    Binding::Item(def_id) => {
                        let def = self.r.defs.get_definition(def_id);

                        if i < segments.len() - 1 {
                            if matches!(
                                def.unwrap().kind,
                                DefKind::TypeParam
                                    | DefKind::Struct
                                    | DefKind::Enum
                                    | DefKind::TypeAlias
                                    | DefKind::Trait
                            ) {
                                self.r.defs.insert_ast_id(path.ast_id, def_id);
                                return;
                            }
                            self.report_unresolved_path(path);
                            return;
                        }
                        self.r.defs.insert_ast_id(path.ast_id, def_id);
                        return;
                    }
                    Binding::Import(import_id) => {
                        let import = self.r.module_arena.get_import(import_id);
                        match &import.resolved_binding {
                            Some(Binding::Module(module_id)) => {
                                current_module = *module_id;
                            }
                            Some(Binding::Item(def_id)) => {
                                let def = self.r.defs.get_definition(*def_id);

                                if i < segments.len() - 1 {
                                    if matches!(
                                        def.unwrap().kind,
                                        DefKind::TypeParam
                                            | DefKind::Struct
                                            | DefKind::Enum
                                            | DefKind::TypeAlias
                                            | DefKind::Trait
                                    ) {
                                        self.r.defs.insert_ast_id(path.ast_id, *def_id);
                                        return;
                                    }
                                    self.report_unresolved_path(path);
                                    return;
                                }
                                self.r.defs.insert_ast_id(path.ast_id, *def_id);
                                return;
                            }
                            Some(Binding::Import(_)) | None => {
                                self.report_unresolved_path(path);
                                return;
                            }
                        }
                    }
                },
                None => {
                    self.report_unresolved_path(path);
                    return;
                }
            }
        }

        if let ModuleKind::Def(def_id) = self.r.module_arena.get(current_module).kind {
            self.r.defs.insert_ast_id(path.ast_id, def_id);
        } else {
            todo!("emit error")
        }
    }

    fn resolve_first_segment(&self, segments: &[AstNode<PathSegment>]) -> (ModuleId, usize) {
        let first_ident = &segments[0].node.ident.node;

        match first_ident.name.as_str() {
            "crate" => (self.r.module_arena.root_id(), 1),
            "super" => {
                let mut current = self.parent;
                loop {
                    let module = self.r.module_arena.get(current);
                    match module.kind {
                        ModuleKind::Def(_) => break,
                        ModuleKind::Block => {
                            current = module.parent().unwrap_or(current);
                        }
                    }
                    if module.parent().is_none() {
                        break;
                    }
                }

                let module = self.r.module_arena.get(current);
                current = module.parent().unwrap_or(current);
                let mut skip = 1;

                for segment in segments.iter().skip(1) {
                    if segment.node.ident.node.name == "super" {
                        let module = self.r.module_arena.get(current);
                        current = module.parent().expect("Add error if super goes beyond root");
                        skip += 1;
                    } else {
                        break;
                    }
                }

                (current, skip)
            }
            "self" => (self.parent, 1),
            _ => (self.parent, 0),
        }
    }

    fn resolve_ident_in_module(&self, module_id: ModuleId, ident: &Ident) -> Option<Binding> {
        let module = self.r.module_arena.get(module_id);
        match module.get(ident) {
            Some(binding) => Some(binding.clone()),
            None => match module.kind {
                ModuleKind::Block => {
                    let parent = module.parent()?;
                    self.resolve_ident_in_module(parent, ident)
                }
                ModuleKind::Def(_) => None,
            },
        }
    }

    fn report_unresolved_path(&mut self, path: &AstNode<Path>) {
        let path_str = path
            .node
            .segments
            .iter()
            .map(|s| s.node.ident.node.name.clone())
            .collect::<Vec<_>>()
            .join("::");

        self.r
            .session
            .push_error(CompilerError::ResolverError(ResolverError::UnresolvedPath {
                src: self.r.session.get_named_source(),
                span: path.span,
                path: path_str,
            }));
    }
}

impl<'a, 'r> visitor::Visitor for LateResolver<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let orig_module = self.parent;
        if let Some(module_id) = self.r.modules.get(&item.ast_id) {
            self.parent = *module_id;
        }
        self.with_rib(RibKind::Item, |this| visitor::walk_item(this, item));
        self.parent = orig_module;
    }

    fn visit_let_stmt(&mut self, let_stmt: &AstNode<LetStmt>) {
        self.with_rib(RibKind::Local, |this| {
            this.resolve_pattern(&let_stmt.node.pat, PatternSource::Normal);
            visit_opt!(this, visit_type, &let_stmt.node.type_annotation);
            visit_opt!(this, visit_expr, &let_stmt.node.expr);
        });
    }

    fn visit_block(&mut self, block: &AstNode<BlockExpr>) {
        let orig_module = self.parent;
        self.parent = *self.r.modules.get(&block.ast_id).unwrap();
        visitor::walk_block(self, block);
        self.parent = orig_module;
    }

    fn visit_pattern(&mut self, pattern: &AstNode<Pattern>) {
        self.resolve_pattern(pattern, PatternSource::Normal);
    }

    fn visit_path(&mut self, path: &AstNode<Path>) {
        self.resolve_path(path);
    }

    fn visit_expr(&mut self, expr: &AstNode<Expr>) {
        match &expr.node {
            Expr::For(for_expr) => {
                self.visit_expr(&for_expr.iterator);
                self.with_rib(RibKind::Local, |this| {
                    this.resolve_pattern(&for_expr.pattern, PatternSource::Normal);
                    this.visit_block(&for_expr.body);
                });
            }
            Expr::Match(match_expr) => {
                self.visit_expr(&match_expr.value);
                for arm in &match_expr.arms {
                    self.with_rib(RibKind::Local, |this| {
                        this.resolve_pattern(&arm.node.pattern, PatternSource::Match);
                        this.visit_expr(&arm.node.body);
                    });
                }
            }
            Expr::Let(let_expr) => {
                self.visit_expr(&let_expr.value);
                self.resolve_pattern(&let_expr.pattern, PatternSource::Match);
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}
