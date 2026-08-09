use crate::Session;
use crate::ast_lowerer::hir::{self, BlockExpr, FnDecl, FnSig, HirId, HirNode, Item, Node, Pattern, Stmt};
use crate::resolver::DefId;
use crate::resolver::defs::DefKind;
use crate::resolver::ribs::{PrimTy, Res};
use crate::resolver::DefId;
use crate::type_checker::ty::{self, CollectedTypes, GenericArg, GenericArgs, GenericParamDef, Ty, TyVarId};
use crate::Session;
use std::assert_matches;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct InferCtxt {
    type_var_map: HashMap<TyVarId, Ty>,
    types: HashMap<HirId, Ty>,
    current_ty_var: u32,
}

impl InferCtxt {
    pub fn next_ty_var(&mut self) -> Ty {
        let ty_var = Ty::TyVar(TyVarId::new(self.current_ty_var));
        self.current_ty_var += 1;
        ty_var
    }
}

pub struct TypeChecker<'hir> {
    session: &'hir Session,
    collected_types: CollectedTypes,
    def_to_hir: &'hir HashMap<DefId, HirId>,
    hir_nodes: &'hir HashMap<HirId, Node>,
    infer_ctxt: InferCtxt,
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
        }
    }

    pub fn check(&mut self) {
        for (def_id, hir_id) in self.def_to_hir {
            let Node::Item(item_kind) = self.hir_nodes.get(hir_id).unwrap() else {
                continue;
            };
            let Item::Fn(FnDecl { def_id: _, sig, body }) = &item_kind.node else {
                continue;
            };

            self.check_fn(*def_id, sig, body);
        }
    }

    fn check_fn(&mut self, def_id: DefId, sig: &HirNode<FnSig>, body: &HirNode<BlockExpr>) {
        let fn_sig_ty = self.collected_types.fn_sig.get(&def_id).unwrap();

        for (param, param_ty) in sig.node.params.iter().zip(fn_sig_ty.params.clone()) {
            self.check_pattern(&param.node.pattern, param_ty);
        }

        self.check_block(body);
        todo!()
    }

    fn check_block(&mut self, block: &HirNode<BlockExpr>) {
        for stmt in &block.node.stmts {
            match &stmt.node {
                Stmt::Let(let_stmt) => {
                    let expected = let_stmt
                        .ty
                        .as_ref()
                        .map_or(self.infer_ctxt.next_ty_var(), |ty| self.lower_ty(ty));

                    if let Some(expr) = &let_stmt.expr {
                        let expr_ty = self.check_expression(expr);
                        self.unify(expr_ty, expected.clone());
                    }

                    self.check_pattern(&let_stmt.pattern, expected);
                }
                Stmt::Item(item) => todo!(),
                Stmt::Semi(expr) => todo!(),
                Stmt::Expr(expr) => todo!(),
            }
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
            Pattern::Path(path) => match &path.node {
                hir::Path::Resolved { res, segments } => {
                    if segments.len() == 1 {
                        assert_matches!(res, Res::Local(_));
                        expected
                    } else {
                        todo!()
                    }
                }
                hir::Path::Unresolved {
                    res,
                    resolved_segments,
                    unresolved_segments,
                } => todo!(),
            },
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

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> Ty {
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

    fn lower_resolved_ty(&self, res: &Res, segments: &[HirNode<hir::PathSegment>]) -> Ty {
        match res {
            Res::Def(def_id, def_kind) => match def_kind {
                DefKind::Struct => {
                    let (last_segment, leading_segments) = segments.split_last().unwrap();
                    // resolved paths can only have modules as leading segments
                    // e.g. S<i32>::Assoc<u32> is not possible
                    self.prohibit_generic_args(leading_segments);

                    let generics = self.generics_of(*def_id);
                    let generic_args = self.generic_args(last_segment);
                    assert_eq!(generics.len(), generic_args.len());

                    for (arg, param) in generics.iter().zip(&generic_args) {
                        match (&arg.kind, param) {
                            (ty::GenericParamKind::Type, GenericArg::Type(_)) => {}
                            (ty::GenericParamKind::Const, GenericArg::Const(_)) => {}
                            _ => panic!("emit error"),
                        }
                    }

                    Ty::Struct(*def_id, generic_args)
                }
                DefKind::Enum => {
                    let (last_segment, leading_segments) = segments.split_last().unwrap();
                    // resolved paths can only have modules as leading segments
                    // e.g. S<i32>::Assoc<u32> is not possible
                    self.prohibit_generic_args(leading_segments);

                    let generics = self.generics_of(*def_id);
                    let generic_args = self.generic_args(last_segment);
                    assert_eq!(generics.len(), generic_args.len());

                    for (arg, param) in generics.iter().zip(&generic_args) {
                        match (&arg.kind, param) {
                            (ty::GenericParamKind::Type, GenericArg::Type(_)) => {}
                            (ty::GenericParamKind::Const, GenericArg::Const(_)) => {}
                            _ => panic!("emit error"),
                        }
                    }

                    Ty::Enum(*def_id, generic_args)
                }
                DefKind::StructField => todo!(),
                DefKind::EnumVariant => todo!(),
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
            Res::SelfTy(self_ty_info) => todo!(),
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

    fn generic_args(&self, segment: &HirNode<hir::PathSegment>) -> GenericArgs {
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
                panic!("implement error variant")
            }
        }
    }

    fn instantiate(&self, ty: &Ty, substs: &[Ty]) -> Ty {
        match ty {
            Ty::Unit | Ty::Bool | Ty::I32 | Ty::U32 | Ty::F64 | Ty::Str | Ty::Never | Ty::TyVar(_) | Ty::Err => {
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
            Ty::GenericParam(index) => substs[*index].clone(),
        }
    }

    fn instantiate_generic_arg(&self, generic_arg: &GenericArg, substs: &[Ty]) -> GenericArg {
        match generic_arg {
            GenericArg::Type(ty) => GenericArg::Type(self.instantiate(ty, substs)),
            GenericArg::Const(_) => todo!(),
        }
    }

    fn check_expression(&self, expr: &HirNode<hir::Expr>) -> Ty {
        match &expr.node {
            hir::Expr::Array(hir_nodes) => todo!(),
            hir::Expr::Struct(struct_expr) => todo!(),
            hir::Expr::Call(call_expr) => todo!(),
            hir::Expr::MethodCall(method_call_expr) => todo!(),
            hir::Expr::Tuple(tuple_expr) => {
                Ty::Tuple(tuple_expr.iter().map(|expr| self.check_expression(expr)).collect())
            }
            hir::Expr::Cast(cast_expr) => todo!(),
            hir::Expr::Return(hir_node) => todo!(),
            hir::Expr::Loop(loop_expr) => todo!(),
            hir::Expr::Assign(assign_expr) => todo!(),
            hir::Expr::Field(field_expr) => todo!(),
            hir::Expr::Index(index_expr) => todo!(),
            hir::Expr::Path(hir_node) => todo!(),
            hir::Expr::AddrOf(hir_node) => todo!(),
            hir::Expr::Break(hir_node) => todo!(),
            hir::Expr::Continue => todo!(),
            hir::Expr::Literal(literal) => match literal {
                hir::Literal::Bool(_) => Ty::Bool,
                hir::Literal::I32(_) => Ty::I32,
                hir::Literal::U32(_) => Ty::U32,
                hir::Literal::F64(_) => Ty::F64,
                hir::Literal::Str(_) => Ty::Str,
                hir::Literal::Unit => Ty::Unit,
            },
            hir::Expr::Binary(binary_expr) => todo!(),
            hir::Expr::Unary(unary_expr) => todo!(),
            hir::Expr::If(if_expr) => todo!(),
            hir::Expr::Block(hir_node) => todo!(),
            hir::Expr::Match(match_expr) => todo!(),
            hir::Expr::Let(let_expr) => todo!(),
            hir::Expr::Err => todo!(),
        }
    }

    fn unify(&mut self, found: Ty, expected: Ty) {
        let found = self.shallow_resolve(found);
        let expected = self.shallow_resolve(expected);

        dbg!(&found, &expected);

        match (found, expected) {
            (Ty::I32, Ty::I32)
            | (Ty::U32, Ty::U32)
            | (Ty::F64, Ty::F64)
            | (Ty::Str, Ty::Str)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Unit, Ty::Unit) => {}
            (Ty::TyVar(id), ty) | (ty, Ty::TyVar(id)) => {
                self.infer_ctxt.type_var_map.insert(id, ty);
            }
            (Ty::Tuple(found_tys), Ty::Tuple(expected_tys)) => {
                for (found, expected) in found_tys.into_iter().zip(expected_tys) {
                    self.unify(found, expected);
                }
            }
            _ => todo!(),
        }
    }

    fn shallow_resolve(&self, ty: Ty) -> Ty {
        match ty {
            Ty::TyVar(ty_var) if let Some(ty) = self.infer_ctxt.type_var_map.get(&ty_var) => {
                self.shallow_resolve(ty.clone())
            }
            _ => ty,
        }
    }
}
