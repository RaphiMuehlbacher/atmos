use crate::error::CompilerError;
use crate::extension::SourceSpanExt;
use crate::parser::ast::{
    AstNode, BlockExpr, Crate, Expr, FnSig, GenericParam, GenericParamKind, Ident, Item, LetStmt, MatchArm, Path,
    PathSegment, Pattern, Stmt, Ty, VariantData,
};
use crate::parser::AstId;
use crate::resolver::collect_defs::DefCollector;
use crate::resolver::defs::{DefId, DefKind, DefinitionMap};
use crate::resolver::modules::{Import, Module};
use crate::resolver::resolutions::ResolutionMap;
use crate::resolver::ribs::{Rib, RibKind};
use crate::resolver::visitor::walk_crate;
use crate::resolver::ResolverError;
use crate::Session;
use std::collections::HashMap;

pub struct Resolver<'ast> {
    session: &'ast Session,
    ast_program: &'ast Crate,

    ribs: Vec<Rib>,
    resolutions: ResolutionMap,
    definitions: DefinitionMap,

    modules: HashMap<AstId, Module<'ast>>,
    unresolved_imports: Vec<Import<'ast>>,
}

impl<'ast> Resolver<'ast> {
    pub fn new(session: &'ast Session, ast_program: &'ast Crate) -> Self {
        let mut resolver = Resolver {
            session,
            ast_program,
            ribs: vec![Rib::item()],
            resolutions: ResolutionMap::default(),
            definitions: DefinitionMap::default(),
            modules: HashMap::new(),
            unresolved_imports: Vec::new(),
        };
        resolver.insert_builtins();
        resolver
    }

    fn insert_builtins(&mut self) {
        let builtins = ["i32", "u32", "f64", "bool", "str"];

        for builtin_name in builtins {
            self.insert_builtin_type(builtin_name);
        }
    }

    fn insert_builtin_type(&mut self, name: &str) {
        let ident = AstNode::new(Ident::new(name.to_string()), miette::SourceSpan::err_span());
        let def_id = self.definitions.insert(ident, DefKind::BuiltinType);
        self.innermost_rib().insert(name.to_string(), def_id);
    }

    pub fn resolve(&mut self) {
        self.collect_definitions(self.ast_program);
    }

    fn collect_definitions(&mut self, krate: &Crate) {
        let mut def_collector = DefCollector::new(self);
        walk_crate(&mut def_collector, krate);
    }

    fn resolve_item(&mut self, item: &AstNode<Item>) {
        match &item.node {
            Item::Fn(fn_decl) => {
                self.resolve_fn_sig(&fn_decl.sig);
                self.resolve_block(&fn_decl.body.node);
            }
            Item::Struct(struct_decl) => {
                for type_param in &struct_decl.generics {
                    self.resolve_type_param(type_param);
                }
                self.resolve_variant(&struct_decl.data);
            }
            Item::Enum(enum_decl) => {
                for type_param in &enum_decl.generics {
                    self.resolve_type_param(type_param);
                }
                for variant in &enum_decl.variants {
                    self.resolve_variant(&variant.node.data);
                }
            }
            Item::Trait(_) => {}
            Item::Mod(_) => {}
            Item::Impl(_) => {}
            Item::ExternFn(_) => {}
            Item::Const(_) => {}
            Item::Use(_) => {}
            Item::TyAlias(_) => {}
            Item::Err => {}
        }
    }

    fn declare_item(&mut self, item: &AstNode<Item>) {
        match &item.node {
            Item::Fn(fn_decl) => {
                self.define_ident(&fn_decl.sig.node.ident, DefKind::Function);
            }
            Item::Struct(struct_decl) => {
                self.define_ident(&struct_decl.ident, DefKind::Struct);
            }
            Item::Enum(enum_decl) => {
                self.define_ident(&enum_decl.ident, DefKind::Enum);
            }
            Item::Trait(trait_decl) => {
                self.define_ident(&trait_decl.ident, DefKind::Trait);
            }
            Item::Mod(_) => {}
            Item::Impl(impl_decl) => {}
            Item::ExternFn(extern_fn_decl) => {
                self.define_ident(&extern_fn_decl.sig.node.ident, DefKind::Function);
            }
            Item::Const(const_decl) => {
                self.define_ident(&const_decl.ident, DefKind::Const);
            }
            Item::Use(use_item) => todo!(),

            Item::TyAlias(ty_alias_decl) => {
                self.define_ident(&ty_alias_decl.ident, DefKind::TypeAlias);
            }
            Item::Err => {}
        }
    }

    fn resolve_fn_sig(&mut self, sig: &AstNode<FnSig>) {
        for type_param in &sig.node.generics {
            self.resolve_type_param(type_param);
        }

        for param in &sig.node.params {
            self.resolve_pattern_with_rib(&param.node.pattern, DefKind::Parameter, RibKind::Local);
            self.resolve_type(&param.node.type_annotation);
        }
    }

    fn resolve_block(&mut self, block: &BlockExpr) {
        self.declare_block(block);
        self.resolve_block_contents(block);
    }

    fn resolve_type(&mut self, ty: &AstNode<Ty>) {
        match &ty.node {
            Ty::Path(path) => self.resolve_path(path),
            Ty::Array(ty, expr) => {
                self.resolve_type(ty);
                self.resolve_expr(expr);
            }
            Ty::Ptr(ty) => self.resolve_type(ty),
            Ty::Fn(param_types, return_ty) => {
                for param in param_types {
                    self.resolve_type(param);
                }
                if let Some(return_ty) = return_ty.as_ref() {
                    self.resolve_type(return_ty);
                }
            }
            Ty::Tuple(types) => {
                for ty in types {
                    self.resolve_type(ty);
                }
            }
            Ty::Paren(ty) => self.resolve_type(ty),
        }
    }

    fn resolve_variant(&mut self, variant: &AstNode<VariantData>) {
        match &variant.node {
            VariantData::Unit => {}
            VariantData::Struct { fields } => {
                for field in fields {
                    self.resolve_type(&field.node.type_annotation);
                }
            }
            VariantData::Tuple { types } => {
                for ty in types {
                    self.resolve_type(ty)
                }
            }
        }
    }
    fn resolve_type_param(&mut self, type_param: &AstNode<GenericParam>) {
        self.define_ident(&type_param.node.ident, DefKind::TypeParam);
        if let GenericParamKind::Const(ty) = &type_param.node.kind {
            self.resolve_type(ty);
        }

        for bound in &type_param.node.bounds {
            self.resolve_path(&bound);
        }
    }

    fn declare_block(&mut self, block: &BlockExpr) {
        for stmt in &block.stmts {
            if let Stmt::Item(item) = &stmt.node {
                self.declare_item(item);
            }
        }
    }

    fn resolve_block_contents(&mut self, block: &BlockExpr) {
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: &AstNode<Stmt>) {
        match &stmt.node {
            Stmt::Item(item) => self.resolve_item(item),
            Stmt::Let(let_stmt) => self.resolve_let_stmt(let_stmt),
            Stmt::Expr(expr) | Stmt::Semi(expr) => self.resolve_expr(expr),
            Stmt::Err => {}
        }
    }

    fn resolve_let_stmt(&mut self, let_stmt: &LetStmt) {
        self.resolve_pattern_with_rib(&let_stmt.pat, DefKind::Variable, RibKind::Local);

        if let Some(expr) = &let_stmt.expr {
            self.resolve_expr(&expr);
        }
    }

    fn resolve_expr(&mut self, expr: &AstNode<Expr>) {
        match &expr.node {
            Expr::Array(array) => {
                for element in &array.expressions {
                    self.resolve_expr(element);
                }
            }
            Expr::Struct(struct_expr) => {
                self.resolve_path(&struct_expr.name);
                for field in &struct_expr.fields {
                    self.resolve_expr(&field.node.expr);
                }
            }
            Expr::Call(call) => {
                self.resolve_expr(&call.callee);
                for arg in &call.arguments {
                    self.resolve_expr(arg);
                }
            }
            Expr::MethodCall(method) => {
                self.resolve_expr(&method.receiver);
                self.resolve_path_segment(&method.name);
                for arg in &method.arguments {
                    self.resolve_expr(arg);
                }
            }
            Expr::Tuple(tuple) => {
                for element in &tuple.expressions {
                    self.resolve_expr(element);
                }
            }
            Expr::Cast(cast) => self.resolve_expr(&cast.expr),
            Expr::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.resolve_expr(&value);
                }
            }
            Expr::While(while_expr) => {
                self.resolve_expr(&while_expr.condition);
                self.resolve_block(&while_expr.body.node);
            }
            Expr::Loop(loop_expr) => self.resolve_block(&loop_expr.body.node),
            Expr::For(for_expr) => {
                self.resolve_expr(&for_expr.iterator);
                self.resolve_pattern_with_rib(&for_expr.pattern, DefKind::Variable, RibKind::Local);
                self.resolve_block(&for_expr.body.node);
            }
            Expr::Assign(assign) => {
                self.resolve_expr(&assign.target);
                self.resolve_expr(&assign.value);
            }
            Expr::AssignOp(assign_op) => {
                self.resolve_expr(&assign_op.target);
                self.resolve_expr(&assign_op.value);
            }
            Expr::FieldAccess(field_access) => self.resolve_expr(&field_access.target),
            Expr::Index(index_expr) => {
                self.resolve_expr(&index_expr.target);
                self.resolve_expr(&index_expr.index);
            }
            Expr::Path(path_expr) => {
                self.resolve_path(&path_expr.path);
            }
            Expr::AddrOf(addr_of) => self.resolve_expr(&addr_of.expr),
            Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.resolve_expr(value);
                }
            }
            Expr::Continue => {}
            Expr::Literal(_) => {}
            Expr::Binary(binary) => {
                self.resolve_expr(&binary.left);
                self.resolve_expr(&binary.right);
            }
            Expr::Unary(unary) => self.resolve_expr(&unary.operand),
            Expr::If(if_expr) => {
                self.resolve_expr(&if_expr.condition);
                self.resolve_block(&if_expr.then_branch.node);
                if let Some(else_branch) = &if_expr.else_branch {
                    self.resolve_block(&else_branch.node);
                }
            }
            Expr::Block(block) => self.resolve_block(block),
            Expr::Match(match_expr) => {
                self.resolve_expr(&match_expr.value);
                for arm in &match_expr.arms {
                    self.resolve_match_arm(arm);
                }
            }
            Expr::Let(let_expr) => {
                self.resolve_expr(&let_expr.value);
                self.resolve_pattern_with_rib(&let_expr.pattern, DefKind::Variable, RibKind::Local);
            }
            Expr::Paren(inner) => self.resolve_expr(inner),
            Expr::Err => {}
        }
    }

    fn resolve_match_arm(&mut self, arm: &AstNode<MatchArm>) {
        self.resolve_pattern_with_rib(&arm.node.pattern, DefKind::Variable, RibKind::Local);
        self.resolve_expr(&arm.node.body);
    }

    fn resolve_pattern_with_rib(&mut self, pattern: &AstNode<Pattern>, binding_kind: DefKind, rib_kind: RibKind) {
        self.push_rib(rib_kind);
        self.resolve_pattern(pattern, binding_kind);
        self.pop_rib();
    }
    fn resolve_pattern(&mut self, pattern: &AstNode<Pattern>, binding_kind: DefKind) {
        match &pattern.node {
            Pattern::Wildcard => {}
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    self.resolve_pattern(pattern, binding_kind);
                }
            }
            Pattern::Path(path) => {
                if path.node.segments.len() == 1 {
                    if let Some(segment) = path.node.segments.first() {
                        self.define_ident(&segment.node.ident, binding_kind.clone());
                    }
                } else {
                    self.resolve_path(path);
                }
            }
            Pattern::Struct(path, fields) => {
                self.resolve_path(path);
                for field in fields {
                    self.resolve_pattern(&field.node.pattern, binding_kind);
                }
            }
            Pattern::TupleStruct(path, patterns) => {
                self.resolve_path(path);
                for pattern in patterns {
                    self.resolve_pattern(pattern, binding_kind);
                }
            }
            Pattern::Tuple(patterns) => {
                for pattern in patterns {
                    self.resolve_pattern(pattern, binding_kind);
                }
            }
            Pattern::Expr(expr) => self.resolve_expr(expr),
            Pattern::Paren(inner) => self.resolve_pattern(inner, binding_kind),
        }
    }

    fn define_ident(&mut self, ident: &AstNode<Ident>, kind: DefKind) {
        if let Some(previous) = self.innermost_rib().get(&ident.node.name) {
            self.session
                .push_error(CompilerError::ResolverError(ResolverError::DuplicateDefinition {
                    src: self.session.get_named_source(),
                    span: ident.span,
                    name: ident.node.name.clone(),
                    previous_span: self.definitions.get(&previous).unwrap().span,
                }));
        }
        let def_id = self.definitions.insert(ident.clone(), kind);
        self.resolutions.insert(ident.ast_id, def_id);
        self.innermost_rib().insert(ident.node.name.clone(), def_id);
    }

    fn resolve_path_segment(&mut self, segment: &AstNode<PathSegment>) {
        let ident = &segment.node.ident;
        let Some(def_id) = self.lookup_rib(&ident.node.name) else {
            self.session
                .push_error(CompilerError::ResolverError(ResolverError::NameNotFound {
                    src: self.session.get_named_source(),
                    span: ident.span,
                    name: ident.node.name.clone(),
                }));
            return;
        };
        self.resolutions.insert(ident.ast_id, def_id);
    }

    fn resolve_path(&mut self, path: &AstNode<Path>) {
        assert_eq!(path.node.segments.len(), 1);

        for segment in &path.node.segments {
            self.resolve_path_segment(segment);
        }
    }

    fn lookup_rib(&self, name: &str) -> Option<DefId> {
        for rib in self.ribs.iter().rev() {
            if let Some(def_id) = rib.get(name) {
                return Some(def_id);
            }
        }
        None
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

    fn with_rib<F>(&mut self, mut f: F, kind: RibKind)
    where
        F: FnMut(&mut Self),
    {
        self.push_rib(kind);
        f(self);
        self.pop_rib();
    }
}
