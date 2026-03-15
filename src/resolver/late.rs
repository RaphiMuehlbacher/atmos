use crate::error::CompilerError;
use crate::parser::ast::{AstNode, BlockExpr, Expr, Ident, Item, LetStmt, Path, PathSegment, Pattern, Ty};
use crate::resolver::defs::DefKind;
use crate::resolver::modules::{Binding, ModuleId, ModuleKind};
use crate::resolver::ribs::{PrimTy, Res, Rib, RibKind, SelfTyInfo};
use crate::resolver::visitor::Visitor;
use crate::resolver::DefId;
use crate::resolver::{visitor, ResolverError};
use crate::{visit_opt, Resolver};
use std::collections::HashSet;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PatternSource {
    Match,  // constructor or binding
    Normal, // binding
}

pub struct LateResolver<'a, 'r> {
    r: &'a mut Resolver<'r>,
    ribs: Vec<Rib>,
    parent: ModuleId,
    self_ty_info: Option<SelfTyInfo>,
}

impl<'a, 'r> LateResolver<'a, 'r> {
    pub fn new(r: &'a mut Resolver<'r>, root: ModuleId) -> Self {
        Self {
            r,
            ribs: vec![Rib::item()],
            parent: root,
            self_ty_info: None,
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
        let mut pattern_bindings: HashSet<Ident> = HashSet::new();
        self.resolve_pattern_inner(pattern, source, &mut pattern_bindings);
    }

    fn resolve_pattern_inner(
        &mut self,
        pattern: &AstNode<Pattern>,
        source: PatternSource,
        pattern_bindings: &mut HashSet<Ident>,
    ) {
        match &pattern.node {
            Pattern::Or(patterns) => {
                let mut first_bindings: Option<HashSet<Ident>> = None;

                for pattern in patterns {
                    let mut alt_bindings = HashSet::new();
                    self.resolve_pattern_inner(pattern, source, &mut alt_bindings);
                    match &first_bindings {
                        None => first_bindings = Some(alt_bindings.clone()),
                        Some(expected) => {
                            for ident in expected.difference(&alt_bindings) {
                                self.r.session.push_error(CompilerError::ResolverError(
                                    ResolverError::VariableNotBoundInPattern {
                                        src: self.r.session.get_named_source(),
                                        span: pattern.span,
                                        name: ident.name.clone(),
                                    },
                                ));
                            }
                        }
                    }
                    pattern_bindings.extend(alt_bindings);
                }
            }
            Pattern::Path(path) => {
                if path.node.segments.len() == 1 {
                    let segment = &path.node.segments[0];
                    let name = &segment.node.ident;

                    if name.node.name == "Self" {
                        self.r
                            .session
                            .push_error(CompilerError::ResolverError(ResolverError::SelfAsBinding {
                                src: self.r.session.get_named_source(),
                                span: name.span,
                            }));
                        return;
                    }

                    if matches!(source, PatternSource::Match) {
                        if let Some(res) = self.lookup_value(&name.node) {
                            match res {
                                Res::Local(_) | Res::PrimTy(_) | Res::SelfTy(_) => {}
                                Res::Def(def_id) => {
                                    self.r.defs.insert_ast_id(path.ast_id, def_id);
                                    return;
                                }
                            }
                        }
                    }

                    if !pattern_bindings.insert(name.node.clone()) {
                        if self.innermost_rib().get(&name.node).is_some() {
                            self.r.session.push_error(CompilerError::ResolverError(
                                ResolverError::DuplicateDefinition {
                                    src: self.r.session.get_named_source(),
                                    span: name.span,
                                    name: name.node.name.clone(),
                                },
                            ));
                        }
                    }

                    self.define_binding(&name, pattern);
                    self.r.defs.insert_resolution(path.ast_id, Res::Local(pattern.ast_id));
                } else {
                    self.resolve_path(path);
                }
            }
            Pattern::Paren(pattern) => self.resolve_pattern_inner(pattern, source, pattern_bindings),
            Pattern::Tuple(patterns) => {
                for pattern in patterns {
                    self.resolve_pattern_inner(pattern, source, pattern_bindings);
                }
            }
            Pattern::TupleStruct(path, patterns) => {
                self.resolve_path(path);
                for pattern in patterns {
                    self.resolve_pattern_inner(pattern, source, pattern_bindings);
                }
            }
            Pattern::Struct(path, struct_fields) => {
                self.resolve_path(path);
                for field in struct_fields {
                    self.visit_ident(&field.node.ident);
                    self.resolve_pattern_inner(&field.node.pattern, source, pattern_bindings);
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
        self.innermost_rib().insert(ident.node.clone(), pattern.ast_id);
    }

    fn get_def_from_ty(&self, ty: &AstNode<Ty>) -> Option<DefId> {
        match &ty.node {
            Ty::Path(path) => {
                if path.node.segments.len() == 1 {
                    let ident = &path.node.segments[0].node.ident.node;
                    if let Some(Res::Def(def_id)) = self.lookup_modules(ident, self.parent) {
                        return Some(def_id);
                    }
                }
                self.r.defs.get_def_from_ast(path.ast_id).copied()
            }
            _ => None,
        }
    }

    fn resolve_path(&mut self, path: &AstNode<Path>) {
        if path.node.segments.is_empty() {
            return;
        }

        let segments = &path.node.segments;
        let Some((mut current_module, segment_start)) = self.resolve_first_segment(segments) else {
            return;
        };

        if segments.len() == 1 && segment_start == 0 {
            let first_ident = &segments[0].node.ident.node;
            match self.lookup_value(first_ident) {
                None => self.report_unresolved_path(path),
                Some(res) => self.r.defs.insert_resolution(path.ast_id, res),
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
                        self.r.defs.insert_resolution(path.ast_id, Res::Def(def_id));
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

        let ModuleKind::Def(def_id) = self.r.module_arena.get(current_module).kind else {
            unreachable!();
        };

        self.r.defs.insert_ast_id(path.ast_id, def_id);
    }

    fn resolve_first_segment(&mut self, segments: &[AstNode<PathSegment>]) -> Option<(ModuleId, usize)> {
        let first_ident = &segments[0].node.ident.node;

        match first_ident.name.as_str() {
            "crate" => Some((self.r.module_arena.root_id(), 1)),
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
                let Some(parent) = module.parent() else {
                    self.r
                        .session
                        .push_error(CompilerError::ResolverError(ResolverError::SuperBeyondRoot {
                            src: self.r.session.get_named_source(),
                            span: segments[0].span,
                        }));
                    return None;
                };
                current = parent;
                let mut skip = 1;

                for segment in segments.iter().skip(1) {
                    if segment.node.ident.node.name == "super" {
                        let module = self.r.module_arena.get(current);
                        let Some(parent) = module.parent() else {
                            self.r
                                .session
                                .push_error(CompilerError::ResolverError(ResolverError::SuperBeyondRoot {
                                    src: self.r.session.get_named_source(),
                                    span: segment.span,
                                }));
                            return None;
                        };
                        current = parent;
                        skip += 1;
                    } else {
                        break;
                    }
                }

                Some((current, skip))
            }
            "self" => Some((self.parent, 1)),
            _ => Some((self.parent, 0)),
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

impl<'a, 'r> Visitor for LateResolver<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let orig_module = self.parent;
        let orig_self_ty_info = self.self_ty_info;

        if let Some(module_id) = self.r.modules.get(&item.ast_id) {
            self.parent = *module_id;
        }

        match &item.node {
            Item::Impl(impl_decl) => {
                let impl_def_id = self.r.defs.get_def_from_ast(item.ast_id).copied();
                if let Some(impl_def) = impl_def_id {
                    let self_ty_def = self.get_def_from_ty(&impl_decl.self_ty);
                    let trait_def = impl_decl
                        .for_trait
                        .as_ref()
                        .and_then(|path| self.r.defs.get_def_from_ast(path.ast_id).copied());
                    self.self_ty_info = Some(SelfTyInfo {
                        self_ty_def,
                        trait_def,
                        impl_or_trait_def: impl_def,
                    });
                }
            }
            Item::Trait(_) => {
                if let Some(trait_def) = self.r.defs.get_def_from_ast(item.ast_id).copied() {
                    self.self_ty_info = Some(SelfTyInfo {
                        self_ty_def: None,
                        trait_def: Some(trait_def),
                        impl_or_trait_def: trait_def,
                    });
                }
            }
            Item::Struct(_) | Item::Enum(_) | Item::TyAlias(_) => {
                if let Some(def_id) = self.r.defs.get_def_from_ast(item.ast_id).copied() {
                    self.self_ty_info = Some(SelfTyInfo {
                        self_ty_def: Some(def_id),
                        trait_def: None,
                        impl_or_trait_def: def_id,
                    });
                }
            }
            _ => {}
        }

        self.with_rib(RibKind::Item, |this| visitor::walk_item(this, item));
        self.parent = orig_module;
        self.self_ty_info = orig_self_ty_info;
    }

    fn visit_let_stmt(&mut self, let_stmt: &AstNode<LetStmt>) {
        self.resolve_pattern(&let_stmt.node.pat, PatternSource::Normal);
        visit_opt!(self, visit_type, &let_stmt.node.type_annotation);
        visit_opt!(self, visit_expr, &let_stmt.node.expr);
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
        if path.node.segments.is_empty() {
            return;
        }

        let first_segment = &path.node.segments[0];
        if first_segment.node.ident.node.name == "Self" {
            if self.self_ty_info.is_none() {
                self.r
                    .session
                    .push_error(CompilerError::ResolverError(ResolverError::SelfOutsideImpl {
                        src: self.r.session.get_named_source(),
                        span: first_segment.span,
                    }));
            }

            for arg in &first_segment.node.args {
                visitor::walk_generic_arg(self, arg);
            }
            for segment in path.node.segments.iter().skip(1) {
                for arg in &segment.node.args {
                    visitor::walk_generic_arg(self, arg);
                }
            }
            return;
        }

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
