use crate::error::CompilerError;
use crate::parser::ast::{AstNode, BlockExpr, Expr, Ident, Item, LetStmt, Path, Pattern};
use crate::resolver::defs::DefKind;
use crate::resolver::modules::{Binding, ModuleId};
use crate::resolver::ribs::{Res, Rib, RibKind};
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
        let mut late_resolver = Self {
            r,
            ribs: vec![Rib::item()],
            parent: root,
        };

        late_resolver.insert_builtins();
        late_resolver
    }

    fn insert_builtins(&mut self) {
        let builtins = ["i32", "u32", "f64", "bool", "str"];

        for builtin_name in builtins {
            self.insert_builtin_type(builtin_name);
        }
    }

    fn insert_builtin_type(&mut self, name: &str) {
        let ast_id = AstNode::err(Ident::new(name.to_string())).ast_id;
        self.r.defs.insert(ast_id, DefKind::BuiltinType);
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
            Pattern::Wildcard => {}
            Pattern::Or(patterns) => {
                // TODO: All alternatives must bind the same names
                for pat in patterns {
                    self.resolve_pattern(pat, source);
                }
            }
            Pattern::Path(path) => {
                if path.node.segments.len() == 1 {
                    // Ident
                    let segment = &path.node.segments[0];
                    let name = &segment.node.ident;

                    if matches!(source, PatternSource::Match) {
                        // In match arms: first try to resolve as a value (enum variant, const)
                        if let Some(_def_id) = self.lookup_value(&name.node) {
                            // Found! This is a constructor pattern (e.g., `None`, `CONST`)
                            // TODO: record resolution
                            return;
                        }
                    }
                    // Not found (or not in match context): this is a binding
                    self.define_binding(&name, pattern);
                } else {
                    // Path
                    self.resolve_path(path);
                }
            }
            Pattern::Struct(path, fields) => {
                // Resolve the struct/variant path
                self.resolve_path(path);
                // Resolve field patterns
                for field in fields {
                    self.resolve_pattern(&field.node.pattern, source);
                }
            }
            Pattern::TupleStruct(path, patterns) => {
                // Resolve the tuple struct/variant path
                self.resolve_path(path);
                // Resolve inner patterns
                for pat in patterns {
                    self.resolve_pattern(pat, source);
                }
            }
            Pattern::Tuple(patterns) => {
                for pat in patterns {
                    self.resolve_pattern(pat, source);
                }
            }
            Pattern::Expr(expr) => {
                // Constant expression pattern (e.g., `1`, `"hello"`)
                visitor::walk_expr(self, expr);
            }
            Pattern::Paren(inner) => {
                self.resolve_pattern(inner, source);
            }
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
    }

    /// Define a new binding in the current rib
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
        // TODO: implement path resolution
        visitor::walk_path(self, path);
    }
}

impl<'a, 'r> visitor::Visitor for LateResolver<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        self.with_rib(RibKind::Item, |this| visitor::walk_item(this, item));
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

    fn visit_path(&mut self, path: &AstNode<Path>) {
        self.resolve_path(path);
    }

    fn visit_expr(&mut self, expr: &AstNode<Expr>) {
        match expr.node {
            Expr::MethodCall(_) => {}
            Expr::Tuple(_) => {}
            Expr::Cast(_) => {}
            Expr::Return(_) => {}
            Expr::While(_) => {}
            Expr::Loop(_) => {}
            Expr::For(_) => {}
            Expr::Assign(_) => {}
            Expr::AssignOp(_) => {}
            Expr::FieldAccess(_) => {}
            Expr::Index(_) => {}
            Expr::Path(_) => visitor::walk_expr(self, expr), // TODO: not sure if this is right
            Expr::AddrOf(_) => {}
            Expr::Break(_) => {}
            Expr::Continue => {}
            Expr::If(_) => {}
            Expr::Block(_) => {}
            Expr::Match(_) => {}
            Expr::Let(_) => {}
            Expr::Err => {}
            _ => visitor::walk_expr(self, expr),
        }
    }
}
