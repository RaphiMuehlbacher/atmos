use crate::Session;
use crate::ast_lowerer::hir::{
    self, AssociatedItemKind, BlockExpr, FnDecl, FnSig, HirId, HirNode, Item, Node, Path, Pattern, Stmt,
};
use crate::error::CompilerError;
use crate::parser::ast::Ident;
use crate::resolver::DefId;
use crate::resolver::defs::DefKind::{self, EnumVariant};
use crate::resolver::ribs::{PrimTy, Res, SelfTyInfo};
use crate::type_checker::error::TypeCheckerError;
use crate::type_checker::ty::{self, CollectedTypes, GenericArg, GenericArgs, GenericParamDef, InferTy, Ty, TyVarId};
use miette::{SourceOffset, SourceSpan};
use std::collections::{HashMap, HashSet};
use std::slice;

#[derive(Default, Debug)]
pub struct InferCtxt {
    type_var_map: HashMap<TyVarId, Ty>,
    types: HashMap<HirId, Ty>,
    current_ty_var: u32,
}

impl InferCtxt {
    pub fn next_ty_var(&mut self) -> Ty {
        let ty_var = Ty::Infer(InferTy::TyVar(TyVarId::new(self.current_ty_var)));
        self.current_ty_var += 1;
        ty_var
    }

    pub fn next_int_var(&mut self) -> Ty {
        let ty_var = Ty::Infer(InferTy::IntVar(TyVarId::new(self.current_ty_var)));
        self.current_ty_var += 1;
        ty_var
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GenericArgPosition {
    // A path used in type position (e.g. `let a: Option<i32> = ...`)`.
    // Omitting generic args here is always an arity error
    Type,
    // A path used in value/expression position (e.g. `Option::Some(3)`)
    // Omitting generic args here will infer them
    Value,
}

pub struct TypeChecker<'hir> {
    session: &'hir Session,
    collected_types: CollectedTypes,
    def_to_hir: &'hir HashMap<DefId, HirId>,
    hir_nodes: &'hir HashMap<HirId, Node>,
    infer_ctxt: InferCtxt,
    return_ty: Option<Ty>,
}

impl<'hir> TypeChecker<'hir> {
    pub fn new(
        session: &'hir Session,
        def_to_hir: &'hir HashMap<DefId, HirId>,
        hir_nodes: &'hir HashMap<HirId, Node>,
        collected_types: CollectedTypes,
    ) -> Self {
        Self {
            session,
            def_to_hir,
            hir_nodes,
            collected_types,
            infer_ctxt: InferCtxt::default(),
            return_ty: None,
        }
    }

    pub fn check(&mut self) {
        for (def_id, hir_id) in self.def_to_hir {
            let (sig, body) = match self.hir_nodes.get(hir_id).unwrap() {
                Node::Item(HirNode {
                    node: Item::Fn(FnDecl { def_id: _, sig, body }),
                    ..
                }) => (sig, body),
                Node::AssociatedItem(assoc_item)
                    if let AssociatedItemKind::Fn(sig, Some(body)) = &assoc_item.node.kind =>
                {
                    (sig, body)
                }
                _ => continue,
            };

            self.check_fn(*def_id, sig, body);
        }
    }

    fn check_fn(&mut self, def_id: DefId, sig: &HirNode<FnSig>, body: &HirNode<BlockExpr>) {
        let fn_sig_ty = self.collected_types.fn_sig.get(&def_id).unwrap().clone();

        for (param, param_ty) in sig.node.params.iter().zip(fn_sig_ty.params) {
            self.check_pattern(&param.node.pattern, param_ty);
        }

        let prev_return_ty = self.return_ty.take();
        self.return_ty = Some(fn_sig_ty.return_ty.clone());

        self.check_block_with_expectation(body, fn_sig_ty.return_ty);

        self.return_ty = prev_return_ty;
    }

    fn check_block(&mut self, block: &HirNode<BlockExpr>) -> Ty {
        let ty_var = self.infer_ctxt.next_ty_var();
        self.check_block_with_expectation(block, ty_var)
    }

    fn check_block_with_expectation(&mut self, block: &HirNode<BlockExpr>, expected: Ty) -> Ty {
        for stmt in &block.node.stmts {
            match &stmt.node {
                Stmt::Let(let_stmt) => {
                    let expected = match &let_stmt.ty {
                        Some(ty) => self.lower_ty(ty),
                        None => self.infer_ctxt.next_ty_var(),
                    };

                    if let Some(expr) = &let_stmt.expr {
                        let expr_ty = self.check_expression(expr);
                        self.unify(expr_ty, expected.clone());
                    }

                    self.check_pattern(&let_stmt.pattern, expected);
                }
                Stmt::Item(item) => todo!(),
                Stmt::Semi(expr) | Stmt::Expr(expr) => {
                    self.check_expression(expr);
                }
            }
        }

        match &block.node.expr {
            Some(expr) => {
                let ty = self.check_expression(expr);
                self.unify(ty.clone(), expected);
                ty
            }
            // TODO: fix when implementing divergence
            None => Ty::Never,
        }
    }

    fn check_pattern(&mut self, pattern: &HirNode<Pattern>, expected: Ty) {
        let ty = match &pattern.node {
            Pattern::Wildcard => expected,
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    let ty_var = self.infer_ctxt.next_ty_var();
                    self.check_pattern(pattern, ty_var);
                }
                expected
            }
            Pattern::Binding(binding) => expected,
            Pattern::Path(path) => todo!(),
            Pattern::Struct(path, pattern_struct_field) => todo!(),
            Pattern::TupleStruct(path, patterns) => todo!(),
            Pattern::Tuple(patterns) => {
                let elements_ty: Vec<Ty> = (0..patterns.len()).map(|_| self.infer_ctxt.next_ty_var()).collect();
                let tuple_ty = Ty::Tuple(elements_ty.clone());

                self.unify(tuple_ty.clone(), expected);
                for (pattern, element) in patterns.iter().zip(elements_ty) {
                    self.check_pattern(pattern, element);
                }
                tuple_ty
            }
            Pattern::Expr(expr) => todo!(),
            Pattern::Err => todo!(),
        };
        self.infer_ctxt.types.insert(pattern.hir_id, ty);
    }

    fn lower_ty(&mut self, hir_ty: &HirNode<hir::Ty>) -> Ty {
        match &hir_ty.node {
            hir::Ty::Path(path) => match &path.node {
                hir::Path::Resolved { res, segments } => self.lower_resolved_ty(res, segments),
                hir::Path::Unresolved {
                    res,
                    resolved_segments,
                    unresolved_segments,
                } => todo!(),
            },
            hir::Ty::Array(ty, _) => Ty::Array(Box::new(self.lower_ty(ty)), todo!()),
            hir::Ty::Ptr(ty) => Ty::Ptr(Box::new(self.lower_ty(ty))),
            hir::Ty::Fn(params, return_ty) => Ty::FnPtr(
                params.iter().map(|param| self.lower_ty(param)).collect(),
                Box::new(return_ty.as_ref().map_or(Ty::Unit, |ty| self.lower_ty(ty))),
            ),
            hir::Ty::Tuple(types) => Ty::Tuple(types.iter().map(|ty| self.lower_ty(ty)).collect()),
            hir::Ty::Err => Ty::Err,
        }
    }

    fn lower_resolved_ty(&mut self, res: &Res, segments: &[HirNode<hir::PathSegment>]) -> Ty {
        match res {
            Res::Def(def_id, def_kind) => match def_kind {
                DefKind::Struct => {
                    let args = self.lower_generic_args(*def_id, segments, GenericArgPosition::Type);
                    Ty::Struct(*def_id, args)
                }
                DefKind::Enum => {
                    let args = self.lower_generic_args(*def_id, segments, GenericArgPosition::Type);
                    Ty::Enum(*def_id, args)
                }
                DefKind::StructField => todo!(),
                DefKind::EnumVariant { .. } => panic!("enum variants in type position not supported"),
                DefKind::Trait => todo!(),
                DefKind::Mod => todo!(),
                DefKind::Impl => todo!(),
                DefKind::AssocFn => todo!(),
                DefKind::ExternFn => todo!(),
                DefKind::Use => todo!(),
                DefKind::Const => todo!(),
                DefKind::GenericParam => todo!(),
                DefKind::TypeAlias => todo!(),
                DefKind::AssocTypeAlias => todo!(),
                DefKind::Function => todo!(),
            },
            Res::Local(ast_id) => todo!(),
            Res::PrimTy(prim_ty) => match prim_ty {
                PrimTy::I32 => ty::Ty::I32,
                PrimTy::U32 => ty::Ty::U32,
                PrimTy::F64 => ty::Ty::F64,
                PrimTy::Bool => ty::Ty::Bool,
                PrimTy::Str => ty::Ty::Str,
            },
            Res::SelfTy(SelfTyInfo {
                impl_or_trait_def: def_id,
                ..
            }) => {
                self.prohibit_generic_args(segments);
                let ty = self.collected_types.type_of.get(def_id).unwrap().clone();
                let generics_count = self.generics_of(*def_id).len();
                let args = self.identity_args(generics_count);
                self.instantiate(&ty, &args)
            }
            Res::Err => todo!(),
        }
    }

    fn generics_of(&self, def_id: DefId) -> Vec<GenericParamDef> {
        let self_generics = self.collected_types.generics_of.get(&def_id).unwrap();
        let mut generics = self_generics.params.clone();
        if let Some(parent_def_id) = self_generics.parent {
            let parent_generics = self.collected_types.generics_of.get(&parent_def_id).unwrap();
            assert_eq!(
                parent_generics.parent, None,
                "We only support associated items nested one level"
            );
            generics.append(&mut parent_generics.params.clone());
        }
        generics
    }

    fn generics_arg_for_segment(
        &mut self,
        def_id: DefId,
        segment: &HirNode<hir::PathSegment>,
        generic_arg_pos: GenericArgPosition,
    ) -> GenericArgs {
        let generics = self.generics_of(def_id);
        let generic_args = self.generic_args(segment);

        match generic_args.len() {
            n if n == generics.len() => {
                for (arg, param) in generics.iter().zip(&generic_args) {
                    match (&arg.kind, param) {
                        (ty::GenericParamKind::Type, GenericArg::Type(_)) => {}
                        (ty::GenericParamKind::Const, GenericArg::Const(_)) => {}
                        _ => {
                            let (expected, found) = match &arg.kind {
                                ty::GenericParamKind::Type => ("type", "const"),
                                ty::GenericParamKind::Const => ("const", "type"),
                            };
                            self.session.push_error(CompilerError::TypeCheckerError(
                                TypeCheckerError::GenericArgKindMismatch {
                                    src: self.session.get_named_source(),
                                    expected_span: segment.span,
                                    found_span: segment.span,
                                    param: format!("{}", arg.index),
                                    expected: expected.to_string(),
                                    found: found.to_string(),
                                },
                            ));
                        }
                    }
                }
                generic_args
            }

            0 if generic_arg_pos == GenericArgPosition::Value => self.identity_args(generics.len()),
            _ => {
                self.session.push_error(CompilerError::TypeCheckerError(
                    TypeCheckerError::GenericArgArityMismatch {
                        src: self.session.get_named_source(),
                        span: segment.span,
                        name: segment.node.ident.node.name.clone(),
                        expected: generics.len(),
                        found: generic_args.len(),
                    },
                ));
                generic_args
            }
        }
    }

    fn lower_generic_args(
        &mut self,
        def_id: DefId,
        segments: &[HirNode<hir::PathSegment>],
        generic_arg_pos: GenericArgPosition,
    ) -> GenericArgs {
        let (last_segment, leading_segments) = segments.split_last().unwrap();
        // resolved paths can only have modules as leading segments
        // e.g. S<i32>::Assoc<u32> is not possible
        self.prohibit_generic_args(leading_segments);
        self.generics_arg_for_segment(def_id, last_segment, generic_arg_pos)
    }

    /// For enum variants
    /// e.g. `Option<i32>::Some(3)
    fn lower_variant_generic_args(&mut self, def_id: DefId, segments: &[HirNode<hir::PathSegment>]) -> GenericArgs {
        // `enum_segments` is both the enum and variant
        let (module_segments, [enum_segment, variant_segment]) = segments.split_last_chunk::<2>().unwrap();

        // modules and variants can't have generics
        self.prohibit_generic_args(module_segments);
        self.prohibit_generic_args(slice::from_ref(variant_segment));

        self.generics_arg_for_segment(def_id, enum_segment, GenericArgPosition::Value)
    }

    fn identity_args(&mut self, count: usize) -> GenericArgs {
        (0..count)
            .map(|_| GenericArg::Type(self.infer_ctxt.next_ty_var()))
            .collect()
    }

    fn generic_args(&mut self, segment: &HirNode<hir::PathSegment>) -> GenericArgs {
        let mut args = vec![];
        for arg in &segment.node.args {
            let arg = match &arg.node {
                hir::GenericArg::Type(ty) => GenericArg::Type(self.lower_ty(ty)),
                hir::GenericArg::Const(_) => todo!(),
            };
            args.push(arg);
        }
        args
    }

    fn prohibit_generic_args(&self, segments: &[HirNode<hir::PathSegment>]) {
        for segment in segments {
            if !segment.node.args.is_empty() {
                self.session.push_error(CompilerError::TypeCheckerError(
                    TypeCheckerError::GenericArgsOnLeadingSegment {
                        src: self.session.get_named_source(),
                        span: segment.span,
                    },
                ));
            }
        }
    }

    fn instantiate(&self, ty: &Ty, substs: &[GenericArg]) -> Ty {
        match ty {
            Ty::Unit | Ty::Bool | Ty::I32 | Ty::U32 | Ty::F64 | Ty::Str | Ty::Never | Ty::Infer(_) | Ty::Err => {
                ty.clone()
            }
            Ty::Array(ty, _) => Ty::Array(Box::new(self.instantiate(ty, substs)), todo!()),
            Ty::Slice(ty) => Ty::Slice(Box::new(self.instantiate(ty, substs))),
            Ty::Tuple(types) => Ty::Tuple(types.iter().map(|ty| self.instantiate(ty, substs)).collect()),
            Ty::Ptr(ty) => Ty::Ptr(Box::new(self.instantiate(ty, substs))),
            Ty::FnPtr(params, return_ty) => Ty::FnPtr(
                params.iter().map(|ty| self.instantiate(ty, substs)).collect(),
                Box::new(self.instantiate(return_ty, substs)),
            ),
            Ty::Fn(def_id, generic_args) => Ty::Fn(
                *def_id,
                generic_args
                    .iter()
                    .map(|arg| self.instantiate_generic_arg(arg, substs))
                    .collect(),
            ),
            Ty::Struct(def_id, generic_args) => Ty::Struct(
                *def_id,
                generic_args
                    .iter()
                    .map(|arg| self.instantiate_generic_arg(arg, substs))
                    .collect(),
            ),
            Ty::Enum(def_id, generic_args) => Ty::Enum(
                *def_id,
                generic_args
                    .iter()
                    .map(|arg| self.instantiate_generic_arg(arg, substs))
                    .collect(),
            ),
            Ty::InherentTyAlias { .. } => todo!(),
            Ty::GenericParam(index) => match &substs[*index] {
                GenericArg::Type(ty) => ty.clone(),
                GenericArg::Const(_) => todo!(),
            },
        }
    }

    fn instantiate_generic_arg(&self, generic_arg: &GenericArg, substs: &[GenericArg]) -> GenericArg {
        match generic_arg {
            GenericArg::Type(ty) => GenericArg::Type(self.instantiate(ty, substs)),
            GenericArg::Const(_) => todo!(),
        }
    }

    fn check_expression(&mut self, expr: &HirNode<hir::Expr>) -> Ty {
        match &expr.node {
            hir::Expr::Array(exprs) => todo!(),
            hir::Expr::Struct(struct_expr) => {
                if let Path::Resolved {
                    res: Res::Def(variant_def_id, DefKind::EnumVariant { enum_def_id }),
                    segments,
                } = &struct_expr.path.node
                {
                    let args = self.lower_generic_args(*enum_def_id, segments, GenericArgPosition::Value);
                    let enum_def = self.collected_types.enums.get(enum_def_id).unwrap().clone();

                    let variant = enum_def
                        .variants
                        .iter()
                        .find(|variant| variant.def_id == *variant_def_id)
                        .unwrap();

                    let fields: HashMap<_, _> = variant
                        .fields
                        .iter()
                        .map(|field| {
                            (
                                &field.ident,
                                self.collected_types.type_of.get(&field.def_id).unwrap().clone(),
                            )
                        })
                        .collect();

                    let mut seen = HashSet::<&Ident>::new();
                    for field_expr in &struct_expr.fields {
                        let ident = &field_expr.node.ident.node;

                        let Some(field_ty) = fields.get(ident) else {
                            self.session
                                .push_error(CompilerError::TypeCheckerError(TypeCheckerError::FieldNotFound {
                                    src: self.session.get_named_source(),
                                    span: field_expr.span,
                                    field: ident.name.clone(),
                                    ty: self.pretty_print_ty(&Ty::Enum(*enum_def_id, args.clone())),
                                }));
                            continue;
                        };
                        let field_ty = field_ty.clone();

                        let field_ty = self.instantiate(&field_ty, &args);

                        if seen.contains(ident) {
                            todo!("emit error for field specified more than once");
                        }
                        seen.insert(ident);

                        let found_ty = self.check_expression(&field_expr.node.expr);
                        self.unify(found_ty, field_ty);
                    }

                    for _ in fields.keys().filter(|ident| !seen.contains(*ident)) {
                        todo!("emit error for missing field")
                    }
                    Ty::Enum(*enum_def_id, args)
                } else {
                    let (def_id, args) = match &struct_expr.path.node {
                        Path::Resolved {
                            res: Res::Def(def_id, DefKind::Struct),
                            segments,
                        } => {
                            let args = self.lower_generic_args(*def_id, segments, GenericArgPosition::Value);
                            (def_id, args)
                        }
                        Path::Resolved {
                            res:
                                Res::SelfTy(SelfTyInfo {
                                    self_ty_def: Some(def_id),
                                    ..
                                }),
                            segments,
                        } => {
                            self.prohibit_generic_args(segments);
                            let generics = self.generics_of(*def_id);

                            let args = self.identity_args(generics.len());
                            (def_id, args)
                        }
                        _ => panic!("shouldn't be possible"),
                    };

                    let struct_def = self.collected_types.structs.get(def_id).unwrap().clone();
                    let struct_fields: HashMap<_, _> = struct_def
                        .fields
                        .into_iter()
                        .map(|field| {
                            (
                                field.ident,
                                self.collected_types.type_of.get(&field.def_id).unwrap().clone(),
                            )
                        })
                        .collect();

                    let mut seen = HashSet::<&Ident>::new();
                    for field_expr in &struct_expr.fields {
                        let ident = &field_expr.node.ident.node;

                        let Some(field_ty) = struct_fields.get(ident) else {
                            self.session
                                .push_error(CompilerError::TypeCheckerError(TypeCheckerError::FieldNotFound {
                                    src: self.session.get_named_source(),
                                    span: field_expr.span,
                                    field: ident.name.clone(),
                                    ty: self.pretty_print_ty(&Ty::Struct(*def_id, args.clone())),
                                }));
                            continue;
                        };
                        let field_ty = field_ty.clone();

                        let field_ty = self.instantiate(&field_ty, &args);

                        if seen.contains(ident) {
                            todo!("emit error for field specified more than once");
                        }
                        seen.insert(ident);

                        let found_ty = self.check_expression(&field_expr.node.expr);
                        self.unify(found_ty, field_ty);
                    }

                    for _ in struct_fields.keys().filter(|ident| !seen.contains(ident)) {
                        todo!("emit error for missing field")
                    }
                    Ty::Struct(*def_id, args)
                }
            }
            hir::Expr::Call(call_expr) => {
                let callee = self.check_expression(&call_expr.callee);

                match callee {
                    Ty::Fn(def_id, generic_args) => {
                        let fn_sig = self.collected_types.fn_sig.get(&def_id).unwrap().clone();

                        if call_expr.args.len() != fn_sig.params.len() {
                            self.session.push_error(CompilerError::TypeCheckerError(
                                TypeCheckerError::FnArgArityMismatch {
                                    src: self.session.get_named_source(),
                                    expected_span: expr.span,
                                    found_span: expr.span,
                                    expected: fn_sig.params.len(),
                                    found: call_expr.args.len(),
                                },
                            ));
                        }

                        for (arg, param) in call_expr.args.iter().zip(&fn_sig.params) {
                            let arg_ty = self.check_expression(arg);
                            let param = self.instantiate(param, &generic_args);
                            self.unify(arg_ty, param.clone());
                        }

                        self.instantiate(&fn_sig.return_ty, &generic_args)
                    }
                    Ty::Struct(def_id, generic_args) => {
                        let struct_def = self.collected_types.structs.get(&def_id).unwrap().clone();

                        let fields: Vec<_> = struct_def
                            .fields
                            .iter()
                            .map(|field| self.collected_types.type_of.get(&field.def_id).unwrap().clone())
                            .collect();

                        if call_expr.args.len() != fields.len() {
                            self.session.push_error(CompilerError::TypeCheckerError(
                                TypeCheckerError::StructArgArityMismatch {
                                    src: self.session.get_named_source(),
                                    expected_span: expr.span,
                                    found_span: expr.span,
                                    expected: fields.len(),
                                    found: call_expr.args.len(),
                                },
                            ));
                        }

                        for (arg, field_ty) in call_expr.args.iter().zip(fields) {
                            let field_ty = self.instantiate(&field_ty, &generic_args);
                            let arg_ty = self.check_expression(arg);
                            self.unify(arg_ty, field_ty);
                        }

                        Ty::Struct(def_id, generic_args)
                    }
                    Ty::Enum(def_id, generic_args) => {
                        let hir::Expr::Path(path) = &call_expr.callee.node else {
                            panic!()
                        };
                        let Path::Resolved {
                            res: Res::Def(variant_def_id, EnumVariant { .. }),
                            segments: _,
                        } = &path.node
                        else {
                            panic!()
                        };

                        let enum_def = self.collected_types.enums.get(&def_id).unwrap().clone();
                        let variant = enum_def
                            .variants
                            .iter()
                            .find(|variant| variant.def_id == *variant_def_id)
                            .unwrap();

                        let fields: Vec<_> = variant
                            .fields
                            .iter()
                            .map(|field| self.collected_types.type_of.get(&field.def_id).unwrap().clone())
                            .collect();

                        if call_expr.args.len() != fields.len() {
                            self.session.push_error(CompilerError::TypeCheckerError(
                                TypeCheckerError::StructArgArityMismatch {
                                    src: self.session.get_named_source(),
                                    expected_span: expr.span,
                                    found_span: expr.span,
                                    expected: fields.len(),
                                    found: call_expr.args.len(),
                                },
                            ));
                        }

                        for (arg, field_ty) in call_expr.args.iter().zip(fields) {
                            let field_ty = self.instantiate(&field_ty, &generic_args);
                            let arg_ty = self.check_expression(arg);
                            self.unify(arg_ty, field_ty);
                        }

                        Ty::Enum(def_id, generic_args)
                    }
                    _ => {
                        self.session
                            .push_error(CompilerError::TypeCheckerError(TypeCheckerError::NotCallable {
                                src: self.session.get_named_source(),
                                span: expr.span,
                                found: self.pretty_print_ty(&callee),
                            }));
                        Ty::Err
                    }
                }
            }
            hir::Expr::MethodCall(method_call) => {
                let receiver = self.check_expression(&method_call.receiver);
                let receiver = self.deeply_resolve(receiver);
                let ident = &method_call.method.node.ident.node;

                match &receiver {
                    Ty::Struct(def_id, args) => {
                        let args = self.lower_generic_args(
                            *def_id,
                            slice::from_ref(&method_call.method),
                            GenericArgPosition::Value,
                        );
                        let impls = self.collected_types.impls_of.get(def_id).unwrap();
                        let assoc_item = impls
                            .iter()
                            .flat_map(|def_id| self.collected_types.assoc_items.get(def_id).unwrap())
                            .find(|assoc| &assoc.ident == ident)
                            .expect("emit error for associated item not found")
                            .clone();

                        let fn_sig = self.collected_types.fn_sig.get(&assoc_item.def_id).unwrap().clone();

                        if method_call.args.len() + 1 != fn_sig.params.len() {
                            self.session.push_error(CompilerError::TypeCheckerError(
                                TypeCheckerError::FnArgArityMismatch {
                                    src: self.session.get_named_source(),
                                    expected_span: expr.span,
                                    found_span: expr.span,
                                    expected: fn_sig.params.len(),
                                    found: method_call.args.len(),
                                },
                            ));
                        }

                        let first_param = self.instantiate(fn_sig.params.first().unwrap(), &args);
                        self.unify(receiver.clone(), first_param);

                        for (arg, param) in method_call.args.iter().zip(fn_sig.params.iter().skip(1)) {
                            let arg_ty = self.check_expression(arg);
                            let param = self.instantiate(param, &args);
                            self.unify(arg_ty, param.clone());
                        }

                        self.instantiate(&fn_sig.return_ty, &args)
                    }
                    Ty::Enum(def_id, args) => todo!(),
                    _ => panic!("emit error that you can only call methods on structs and enums"),
                }
            }
            hir::Expr::Tuple(tuple_expr) => {
                Ty::Tuple(tuple_expr.iter().map(|expr| self.check_expression(expr)).collect())
            }
            hir::Expr::Cast(cast_expr) => todo!(),
            hir::Expr::Return(expr) => {
                let return_ty = expr.as_ref().map_or(Ty::Unit, |expr| self.check_expression(expr));
                self.unify(return_ty, self.return_ty.clone().unwrap());
                Ty::Never
            }
            hir::Expr::Loop(loop_expr) => todo!(),
            hir::Expr::Assign(assign_expr) => {
                let lhs = self.check_expression(&assign_expr.lhs);
                let rhs = self.check_expression(&assign_expr.rhs);
                self.unify(lhs.clone(), rhs);
                lhs
            }
            hir::Expr::Field(field_expr) => {
                let base = self.check_expression(&field_expr.base);
                let base = self.deeply_resolve(base);
                let base_str = self.pretty_print_ty(&base);

                let Ty::Struct(def_id, args) = base else {
                    self.session.push_error(CompilerError::TypeCheckerError(
                        TypeCheckerError::ExpectedStructInFieldAccess {
                            src: self.session.get_named_source(),
                            span: field_expr.field.span,
                            found: base_str,
                        },
                    ));
                    return Ty::Err;
                };
                let fields = &self.collected_types.structs.get(&def_id).unwrap().fields;

                let Some(field) = fields.iter().find(|field| field.ident == field_expr.field.node) else {
                    self.session
                        .push_error(CompilerError::TypeCheckerError(TypeCheckerError::FieldNotFound {
                            src: self.session.get_named_source(),
                            span: field_expr.field.span,
                            field: field_expr.field.node.name.clone(),
                            ty: base_str,
                        }));
                    return Ty::Err;
                };
                let field_def_id = field.def_id;

                let field_ty = self.collected_types.type_of.get(&field_def_id).unwrap().clone();
                self.instantiate(&field_ty, &args)
            }
            hir::Expr::Index(index_expr) => todo!(),
            hir::Expr::Path(path) => match &path.node {
                hir::Path::Resolved { res, segments } => {
                    if segments.len() == 1
                        && let Res::Local(hir_id) = res
                    {
                        self.infer_ctxt.types.get(hir_id).unwrap().clone()
                    } else {
                        match res {
                            Res::Local(_) => todo!(),
                            Res::Def(def_id, DefKind::Struct) => {
                                // Unit structs can't have generics
                                // but tuple structs use `check_expression` for the callee
                                let args = self.lower_generic_args(*def_id, segments, GenericArgPosition::Value);
                                Ty::Struct(*def_id, args)
                            }
                            Res::Def(def_id, DefKind::Function) => {
                                let args = self.lower_generic_args(*def_id, segments, GenericArgPosition::Value);
                                Ty::Fn(*def_id, args)
                            }
                            Res::Def(_, DefKind::EnumVariant { enum_def_id }) => {
                                let args = self.lower_variant_generic_args(*enum_def_id, segments);
                                Ty::Enum(*enum_def_id, args)
                            }
                            Res::Def(def_id, def_kind) => todo!(),
                            Res::PrimTy(prim_ty) => {
                                self.session.push_error(CompilerError::TypeCheckerError(
                                    TypeCheckerError::ExpectedValueType {
                                        src: self.session.get_named_source(),
                                        span: path.span,
                                    },
                                ));
                                Ty::Err
                            }
                            Res::SelfTy(self_ty_info) => todo!(),
                            Res::Err => todo!(),
                        }
                    }
                }
                hir::Path::Unresolved {
                    res,
                    resolved_segments,
                    unresolved_segments,
                } => match res {
                    Res::Def(def_id, DefKind::Struct) => {
                        assert_eq!(
                            unresolved_segments.len(),
                            1,
                            "currently associated items can only have one segment"
                        );
                        let ident = &unresolved_segments[0].node.ident.node;

                        let impls = self.collected_types.impls_of.get(def_id).unwrap();

                        let assoc_item = impls
                            .iter()
                            .flat_map(|def_id| self.collected_types.assoc_items.get(def_id).unwrap())
                            .find(|assoc| &assoc.ident == ident)
                            .expect("emit error for associated item not found")
                            .clone();

                        // TODO: for now only associated functions
                        let args =
                            self.lower_generic_args(assoc_item.def_id, unresolved_segments, GenericArgPosition::Value);
                        Ty::Fn(assoc_item.def_id, args)
                    }
                    Res::SelfTy(SelfTyInfo {
                        impl_or_trait_def: def_id,
                        ..
                    }) => {
                        match unresolved_segments.len() {
                            0 => {
                                let ty = self.collected_types.type_of.get(def_id).unwrap().clone();
                                let generics_count = self.generics_of(*def_id).len();
                                let args = self.identity_args(generics_count);
                                self.instantiate(&ty, &args)
                            }
                            1 => {
                                let item = self
                                    .collected_types
                                    .assoc_items
                                    .get(def_id)
                                    .unwrap()
                                    .iter()
                                    .find(|assoc| assoc.ident == unresolved_segments[0].node.ident.node)
                                    .map(|assoc_def_id| self.collected_types.type_of.get(&assoc_def_id.def_id).unwrap())
                                    .unwrap();

                                // TODO: for now only associated functions
                                let Ty::Fn(def_id, _) = *item else { panic!() };
                                let count = self.generics_of(def_id).len();
                                let args = self.identity_args(count);

                                Ty::Fn(def_id, args)
                            }
                            _ => panic!("currently only one unresolved segment supported"),
                        }
                    }
                    _ => todo!(),
                },
            },
            hir::Expr::AddrOf(expr) => self.check_expression(expr),
            hir::Expr::Break(expr) => expr.as_ref().map_or(Ty::Unit, |expr| self.check_expression(expr)),
            hir::Expr::Continue => Ty::Unit,
            hir::Expr::Literal(literal) => match literal {
                hir::Literal::Bool(_) => Ty::Bool,
                hir::Literal::Int(int_kind) => match int_kind {
                    hir::IntKind::Signed(_) => Ty::I32,
                    hir::IntKind::Unsigned(_) => Ty::U32,
                    hir::IntKind::Unsuffixed(_) => self.infer_ctxt.next_int_var(),
                },
                hir::Literal::F64(_) => Ty::F64,
                hir::Literal::Str(_) => Ty::Str,
                hir::Literal::Unit => Ty::Unit,
            },
            hir::Expr::Binary(binary_expr) => {
                // TODO: change to overloadable trait
                let lhs = self.check_expression(&binary_expr.lhs);
                let rhs = self.check_expression(&binary_expr.rhs);
                self.unify(lhs.clone(), rhs);
                lhs
            }
            hir::Expr::Unary(unary_expr) => todo!(),
            hir::Expr::If(if_expr) => {
                let condition = self.check_expression(&if_expr.condition);
                self.unify(condition, Ty::Bool);

                let then_branch = self.check_block(&if_expr.then_branch);
                let else_ty = match &if_expr.else_branch {
                    Some(block) => self.check_block(block),
                    None => Ty::Never,
                };
                self.unify(then_branch.clone(), else_ty);
                then_branch
            }
            hir::Expr::Block(block) => self.check_block(block),
            hir::Expr::Match(match_expr) => todo!(),
            hir::Expr::Let(let_expr) => todo!(),
            hir::Expr::Err => todo!(),
        }
    }

    fn unify(&mut self, found: Ty, expected: Ty) {
        let found = self.shallow_resolve(found);
        let expected = self.shallow_resolve(expected);

        match (found, expected) {
            (Ty::I32, Ty::I32)
            | (Ty::U32, Ty::U32)
            | (Ty::F64, Ty::F64)
            | (Ty::Str, Ty::Str)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Unit, Ty::Unit) => {}
            (Ty::Infer(InferTy::IntVar(found)), Ty::Infer(InferTy::IntVar(expected))) => {
                self.infer_ctxt
                    .type_var_map
                    .insert(found, Ty::Infer(InferTy::IntVar(expected)));
            }
            (Ty::Infer(InferTy::TyVar(id)), ty) | (ty, Ty::Infer(InferTy::TyVar(id))) => {
                self.infer_ctxt.type_var_map.insert(id, ty);
            }
            (Ty::Infer(InferTy::IntVar(id)), Ty::I32) | (Ty::I32, Ty::Infer(InferTy::IntVar(id))) => {
                self.infer_ctxt.type_var_map.insert(id, Ty::I32);
            }
            (Ty::Infer(InferTy::IntVar(id)), Ty::U32) | (Ty::U32, Ty::Infer(InferTy::IntVar(id))) => {
                self.infer_ctxt.type_var_map.insert(id, Ty::U32);
            }
            (Ty::Tuple(found_tys), Ty::Tuple(expected_tys)) => {
                for (found, expected) in found_tys.into_iter().zip(expected_tys) {
                    self.unify(found, expected);
                }
            }
            (Ty::Struct(found_def_id, generic_args), Ty::Struct(expected_def_id, generic_params))
                if found_def_id == expected_def_id =>
            {
                for (generic_arg, generic_param) in generic_args.into_iter().zip(generic_params) {
                    match (generic_arg, generic_param) {
                        (GenericArg::Type(arg), GenericArg::Type(param)) => self.unify(arg, param),
                        (GenericArg::Const(_), GenericArg::Const(_)) => todo!(),
                        _ => panic!("should be filtered out before"),
                    }
                }
            }
            (Ty::Enum(found_def_id, generic_args), Ty::Enum(expected_def_id, generic_params))
                if found_def_id == expected_def_id =>
            {
                for (generic_arg, generic_param) in generic_args.into_iter().zip(generic_params) {
                    match (generic_arg, generic_param) {
                        (GenericArg::Type(arg), GenericArg::Type(param)) => self.unify(arg, param),
                        (GenericArg::Const(_), GenericArg::Const(_)) => todo!(),
                        _ => panic!("should be filtered out before"),
                    }
                }
            }
            (Ty::GenericParam(found_idx), Ty::GenericParam(expected_idx)) => {
                if found_idx != expected_idx {
                    panic!(
                        "unification failed: found: GenericParam({found_idx}), expected: GenericParam({expected_idx})"
                    );
                }
            }
            (found, expected) => {
                self.session
                    .push_error(CompilerError::TypeCheckerError(TypeCheckerError::TypeMismatch {
                        src: self.session.get_named_source(),
                        expected_span: SourceSpan::new(SourceOffset::from(0), 0),
                        found_span: SourceSpan::new(SourceOffset::from(0), 0),
                        expected: self.pretty_print_ty(&expected),
                        found: self.pretty_print_ty(&found),
                    }));
            }
        }
    }

    fn shallow_resolve(&self, ty: Ty) -> Ty {
        match ty {
            Ty::Infer(InferTy::TyVar(ty_var) | InferTy::IntVar(ty_var))
                if let Some(ty) = self.infer_ctxt.type_var_map.get(&ty_var) =>
            {
                self.shallow_resolve(ty.clone())
            }
            _ => ty,
        }
    }

    fn deeply_resolve(&self, ty: Ty) -> Ty {
        let ty = self.shallow_resolve(ty);

        match ty {
            Ty::Array(ty, expr) => Ty::Array(Box::new(self.deeply_resolve(*ty)), expr),
            Ty::Slice(ty) => Ty::Slice(Box::new(self.deeply_resolve(*ty))),
            Ty::Tuple(types) => Ty::Tuple(types.into_iter().map(|ty| self.deeply_resolve(ty)).collect()),
            Ty::Ptr(ty) => Ty::Ptr(Box::new(self.deeply_resolve(*ty))),
            Ty::FnPtr(params, return_ty) => {
                let params = params.into_iter().map(|param| self.deeply_resolve(param)).collect();
                let return_ty = Box::new(self.deeply_resolve(*return_ty));
                Ty::FnPtr(params, return_ty)
            }
            Ty::Fn(def_id, args) => Ty::Fn(def_id, self.deeply_resolve_args(args)),
            Ty::Struct(def_id, args) => Ty::Struct(def_id, self.deeply_resolve_args(args)),
            Ty::Enum(def_id, args) => Ty::Enum(def_id, self.deeply_resolve_args(args)),
            Ty::InherentTyAlias {
                candidates,
                ident,
                resolved_args,
                unresolved_args,
            } => todo!(),
            Ty::Err => panic!(),
            Ty::Unit
            | Ty::Bool
            | Ty::I32
            | Ty::U32
            | Ty::F64
            | Ty::Str
            | Ty::Never
            | Ty::GenericParam(_)
            | Ty::Infer(_) => ty,
        }
    }

    fn deeply_resolve_args(&self, args: GenericArgs) -> GenericArgs {
        args.into_iter()
            .map(|arg| match arg {
                GenericArg::Type(ty) => GenericArg::Type(self.deeply_resolve(ty)),
                GenericArg::Const(_) => todo!(),
            })
            .collect()
    }

    fn pretty_print_ty(&self, ty: &Ty) -> String {
        match ty {
            Ty::Unit => "()".to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::I32 => "i32".to_string(),
            Ty::U32 => "u32".to_string(),
            Ty::F64 => "f64".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Never => "!".to_string(),
            Ty::Infer(InferTy::IntVar(_)) => "{integer}".to_string(),
            Ty::Infer(InferTy::TyVar(_)) => "{unknown}".to_string(),
            Ty::Array(ty, _) => format!("[{}; N]", self.pretty_print_ty(ty)),
            Ty::Slice(ty) => format!("[{}]", self.pretty_print_ty(ty)),
            Ty::Tuple(types) if types.is_empty() => "()".to_string(),
            Ty::Tuple(types) => {
                let inner = types
                    .iter()
                    .map(|t| self.pretty_print_ty(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({inner})")
            }
            Ty::Ptr(ty) => format!("&{}", self.pretty_print_ty(ty)),
            Ty::FnPtr(params, ret) => {
                let params = params
                    .iter()
                    .map(|t| self.pretty_print_ty(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params, self.pretty_print_ty(ret))
            }
            Ty::Fn(def_id, args) => format!("fn({:?})<{}>", def_id, self.pretty_print_args(args)),
            Ty::Struct(def_id, args) => format!("Struct {:?}<{}>", def_id, self.pretty_print_args(args)),
            Ty::Enum(def_id, args) => format!("Enum {:?}<{}>", def_id, self.pretty_print_args(args)),
            Ty::GenericParam(idx) => format!("T{idx}"),
            Ty::Err => "{err}".to_string(),
            Ty::InherentTyAlias { .. } => "{alias}".to_string(),
        }
    }

    fn pretty_print_args(&self, args: &[GenericArg]) -> String {
        args.iter()
            .map(|arg| match arg {
                GenericArg::Type(ty) => self.pretty_print_ty(ty),
                GenericArg::Const(_) => "_".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}
