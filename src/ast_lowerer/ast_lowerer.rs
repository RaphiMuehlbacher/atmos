use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::HirNode;
use crate::parser::ast;
use crate::parser::ast::AstNode;
use crate::resolver::defs::DefinitionMap;

pub struct AstLowerer<'ast> {
    ast: &'ast ast::Crate,
    defs: &'ast DefinitionMap,
}

impl<'ast> AstLowerer<'ast> {
    pub fn new(defs: &'ast DefinitionMap, ast: &'ast ast::Crate) -> Self {
        Self { ast, defs }
    }

    pub fn lower(&mut self) -> hir::Crate {
        let items = self
            .ast
            .items
            .iter()
            .filter(|item| !matches!(item.node, ast::Item::Use(_)))
            .map(|item| self.lower_item(item))
            .collect();

        hir::Crate {
            items,
            span: self.ast.span,
        }
    }

    fn lower_item(&mut self, item: &AstNode<ast::Item>) -> HirNode<hir::Item> {
        let hir_item = match &item.node {
            ast::Item::Fn(fn_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();

                let sig = self.lower_fn_sig(&fn_item.sig);
                let body = self.lower_block_expr(&fn_item.body);

                hir::Item::Fn(hir::FnDecl { def_id, sig, body })
            }
            ast::Item::Struct(struct_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = struct_item.ident.clone().into();
                let generics = struct_item
                    .generics
                    .iter()
                    .map(|g| self.lower_generic_param(g))
                    .collect();
                let data = self.lower_variant_data(&struct_item.data);
                hir::Item::Struct(hir::StructDecl {
                    def_id,
                    ident,
                    generics,
                    data,
                })
            }
            ast::Item::Enum(enum_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = enum_item.ident.clone().into();
                let generics = enum_item.generics.iter().map(|g| self.lower_generic_param(g)).collect();
                let variants = enum_item.variants.iter().map(|v| self.lower_enum_variant(v)).collect();
                hir::Item::Enum(hir::EnumDecl {
                    def_id,
                    ident,
                    generics,
                    variants,
                })
            }
            ast::Item::Trait(trait_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = trait_item.ident.clone().into();
                let generics = trait_item
                    .generics
                    .iter()
                    .map(|g| self.lower_generic_param(g))
                    .collect();
                let items = trait_item.items.iter().map(|i| self.lower_associated_item(i)).collect();
                hir::Item::Trait(hir::TraitDecl {
                    def_id,
                    ident,
                    generics,
                    items,
                })
            }
            ast::Item::Mod(mod_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = mod_item.ident.clone().into();
                let items = mod_item.items.iter().map(|i| self.lower_item(i)).collect();
                hir::Item::Mod(hir::ModDecl { def_id, ident, items })
            }
            ast::Item::Impl(impl_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let generics = impl_item.generics.iter().map(|g| self.lower_generic_param(g)).collect();
                let self_ty = self.lower_type(&impl_item.self_ty);
                let of_trait = impl_item.for_trait.as_ref().map(|t| self.lower_path(t));
                let items = impl_item.items.iter().map(|i| self.lower_associated_item(i)).collect();
                hir::Item::Impl(hir::ImplDecl {
                    def_id,
                    generics,
                    self_ty,
                    of_trait,
                    items,
                })
            }
            ast::Item::ExternFn(extern_fn_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let sig = self.lower_fn_sig(&extern_fn_item.sig);
                hir::Item::ExternFn(hir::ExternFnDecl { def_id, sig })
            }
            ast::Item::Const(const_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = const_item.ident.clone().into();
                let generics = const_item
                    .generics
                    .iter()
                    .map(|g| self.lower_generic_param(g))
                    .collect();
                let ty = const_item.type_annotation.as_ref().map(|t| self.lower_type(t));
                let expr = self.lower_expr(&const_item.expr);
                hir::Item::Const(hir::ConstDecl {
                    def_id,
                    ident,
                    generics,
                    ty,
                    expr,
                })
            }
            ast::Item::Use(_) => unreachable!("Should already be filtered out"),
            ast::Item::TyAlias(ty_alias_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = ty_alias_item.ident.clone().into();
                let generics = ty_alias_item
                    .generics
                    .iter()
                    .map(|g| self.lower_generic_param(g))
                    .collect();
                let ty = ty_alias_item.ty.as_ref().map(|t| self.lower_type(t));
                hir::Item::TyAlias(hir::TyAlias {
                    def_id,
                    ident,
                    generics,
                    ty,
                })
            }
            ast::Item::Err => unreachable!(),
        };

        HirNode::new(hir_item, item.span)
    }

    fn lower_fn_sig(&mut self, sig: &AstNode<ast::FnSig>) -> HirNode<hir::FnSig> {
        let ident = sig.node.ident.clone().into();
        let generics = sig
            .node
            .generics
            .iter()
            .map(|generic| self.lower_generic_param(generic))
            .collect();

        let params = sig.node.params.iter().map(|param| self.lower_param(param)).collect();
        let return_ty = sig.node.return_ty.as_ref().map(|return_ty| self.lower_type(return_ty));

        HirNode::new(
            hir::FnSig {
                ident,
                generics,
                params,
                return_ty,
            },
            sig.span,
        )
    }

    fn lower_param(&mut self, param: &AstNode<ast::Param>) -> HirNode<hir::Param> {
        let pattern = self.lower_pattern(&param.node.pattern);
        let type_annotation = self.lower_type(&param.node.type_annotation);
        HirNode::new(
            hir::Param {
                pattern,
                type_annotation,
            },
            param.span,
        )
    }

    fn lower_generic_param(&mut self, generic_param: &AstNode<ast::GenericParam>) -> HirNode<hir::GenericParam> {
        let def_id = *self.defs.ast_to_def.get(&generic_param.ast_id).unwrap();
        let ident = generic_param.node.ident.clone().into();
        let bounds = generic_param
            .node
            .bounds
            .iter()
            .map(|bound| self.lower_path(&bound))
            .collect();
        let kind = match &generic_param.node.kind {
            ast::GenericParamKind::Const(ty) => hir::GenericParamKind::Const(self.lower_type(&ty)),
            ast::GenericParamKind::Type => hir::GenericParamKind::Type,
        };

        HirNode::new(
            hir::GenericParam {
                def_id,
                ident,
                bounds,
                kind,
            },
            generic_param.span,
        )
    }

    fn lower_path(&mut self, path: &AstNode<ast::Path>) -> HirNode<hir::Path> {
        let lowered_path = match self.defs.partial_res.get(&path.ast_id) {
            Some(partial_res) => {
                let res = partial_res.base_res();
                let resolved_segments = path.node.segments.len() - partial_res.unresolved_segments();
                let lowered_segments = self.lower_segments(&path.node.segments);

                hir::Path::Unresolved {
                    res,
                    resolved_segments: lowered_segments[0..resolved_segments].to_vec(),
                    unresolved_segments: lowered_segments[resolved_segments..].to_vec(),
                }
            }

            None => {
                let res = self.defs.get_resolution(path.ast_id).unwrap().clone();
                let segments = self.lower_segments(&path.node.segments);
                hir::Path::Resolved { res, segments }
            }
        };
        HirNode::new(lowered_path, path.span)
    }

    fn lower_segments(&mut self, segments: &Vec<AstNode<ast::PathSegment>>) -> Vec<HirNode<hir::PathSegment>> {
        segments.iter().map(|segment| self.lower_segment(&segment)).collect()
    }
    fn lower_segment(&mut self, segment: &AstNode<ast::PathSegment>) -> HirNode<hir::PathSegment> {
        let ident = segment.node.ident.clone().into();
        let args = segment
            .node
            .args
            .iter()
            .map(|arg| self.lower_generic_arg(arg))
            .collect();

        HirNode::new(hir::PathSegment { ident, args }, segment.span)
    }

    fn lower_generic_arg(&mut self, generic_arg: &AstNode<ast::GenericArg>) -> HirNode<hir::GenericArg> {
        let arg = match &generic_arg.node {
            ast::GenericArg::Type(ty) => hir::GenericArg::Type(self.lower_type(ty)),
            ast::GenericArg::Const(expr) => hir::GenericArg::Const(Box::new(self.lower_expr(expr))),
        };
        HirNode::new(arg, generic_arg.span)
    }

    fn lower_type(&mut self, ty: &AstNode<ast::Ty>) -> HirNode<hir::Ty> {
        let hir_ty = match &ty.node {
            ast::Ty::Path(path) => hir::Ty::Path(self.lower_path(path)),
            ast::Ty::Array(ty, expr) => hir::Ty::Array(Box::new(self.lower_type(ty)), Box::new(self.lower_expr(expr))),
            ast::Ty::Ptr(ty) => hir::Ty::Ptr(Box::new(self.lower_type(ty))),
            ast::Ty::Fn(param_tys, return_ty) => {
                let param_tys = param_tys.iter().map(|param_ty| self.lower_type(param_ty)).collect();
                let return_ty = return_ty
                    .as_ref()
                    .as_ref()
                    .map(|return_ty| Box::new(self.lower_type(return_ty)));
                hir::Ty::Fn(param_tys, return_ty)
            }
            ast::Ty::Tuple(types) => {
                let types = types.iter().map(|ty| self.lower_type(ty)).collect();
                hir::Ty::Tuple(types)
            }
            ast::Ty::Paren(ty) => self.lower_type(ty).node,
        };
        HirNode::new(hir_ty, ty.span)
    }

    fn lower_stmt(&mut self, stmt: &AstNode<ast::Stmt>) -> HirNode<hir::Stmt> {
        let hir_stmt = match &stmt.node {
            ast::Stmt::Item(item) => hir::Stmt::Item(self.lower_item(item)),
            ast::Stmt::Let(let_stmt) => {
                let pattern = self.lower_pattern(&let_stmt.pat);
                let ty = let_stmt.type_annotation.as_ref().map(|ty| self.lower_type(&ty));
                let expr = let_stmt.expr.as_ref().map(|expr| Box::new(self.lower_expr(&expr)));

                hir::Stmt::Let(hir::LetStmt { pattern, ty, expr })
            }
            ast::Stmt::Expr(expr) => hir::Stmt::Expr(self.lower_expr(expr)),
            ast::Stmt::Semi(expr) => hir::Stmt::Semi(self.lower_expr(expr)),
            ast::Stmt::Err => unreachable!(),
        };

        HirNode::new(hir_stmt, stmt.span)
    }

    fn lower_expr(&mut self, expr: &AstNode<ast::Expr>) -> HirNode<hir::Expr> {
        let hir_expr = match &expr.node {
            ast::Expr::Array(array_expr) => {
                let exprs = array_expr.expressions.iter().map(|e| self.lower_expr(e)).collect();
                hir::Expr::Array(exprs)
            }
            ast::Expr::Struct(struct_expr) => {
                let path = self.lower_path(&struct_expr.name);
                let fields = struct_expr
                    .fields
                    .iter()
                    .map(|f| self.lower_struct_expr_field(f))
                    .collect();
                hir::Expr::Struct(hir::StructExpr { path, fields })
            }
            ast::Expr::Call(call_expr) => {
                let callee = Box::new(self.lower_expr(&call_expr.callee));
                let args = call_expr.arguments.iter().map(|arg| self.lower_expr(arg)).collect();
                hir::Expr::Call(hir::CallExpr { callee, args })
            }
            ast::Expr::MethodCall(method_call_expr) => {
                let receiver = Box::new(self.lower_expr(&method_call_expr.receiver));
                let method = self.lower_segment(&method_call_expr.name);
                let args = method_call_expr
                    .arguments
                    .iter()
                    .map(|arg| self.lower_expr(arg))
                    .collect();
                hir::Expr::MethodCall(hir::MethodCallExpr { receiver, method, args })
            }
            ast::Expr::Tuple(tuple_expr) => {
                let exprs = tuple_expr.expressions.iter().map(|e| self.lower_expr(e)).collect();
                hir::Expr::Tuple(exprs)
            }
            ast::Expr::Cast(cast_expr) => {
                let expr = Box::new(self.lower_expr(&cast_expr.expr));
                let ty = self.lower_type(&cast_expr.ty);
                hir::Expr::Cast(hir::CastExpr { expr, ty })
            }
            ast::Expr::Return(return_expr) => {
                let value = return_expr.value.as_ref().map(|e| Box::new(self.lower_expr(e)));
                hir::Expr::Return(value)
            }
            ast::Expr::While(while_expr) => {
                // Desugar while to loop + if + break
                // while cond { body } => loop { if cond { body } else { break } }
                let condition = Box::new(self.lower_expr(&while_expr.condition));
                let then_branch = self.lower_block_expr(&while_expr.body);
                let else_branch = HirNode::new(
                    hir::BlockExpr {
                        stmts: vec![HirNode::new(
                            hir::Stmt::Expr(HirNode::new(hir::Expr::Break(None), expr.span)),
                            expr.span,
                        )],
                    },
                    expr.span,
                );
                let if_expr = hir::Expr::If(hir::IfExpr {
                    condition,
                    then_branch,
                    else_branch: Some(else_branch),
                });
                let body = HirNode::new(
                    hir::BlockExpr {
                        stmts: vec![HirNode::new(
                            hir::Stmt::Expr(HirNode::new(if_expr, expr.span)),
                            expr.span,
                        )],
                    },
                    expr.span,
                );
                hir::Expr::Loop(hir::LoopExpr { body })
            }
            ast::Expr::Loop(loop_expr) => {
                let body = self.lower_block_expr(&loop_expr.body);
                hir::Expr::Loop(hir::LoopExpr { body })
            }
            ast::Expr::For(for_expr) => {
                todo!("for loop desugaring")
            }
            ast::Expr::Assign(assign_expr) => {
                let lhs = Box::new(self.lower_expr(&assign_expr.target));
                let rhs = Box::new(self.lower_expr(&assign_expr.value));
                hir::Expr::Assign(hir::AssignExpr { lhs, rhs })
            }
            ast::Expr::AssignOp(assign_op_expr) => {
                // Desugar `a += b` to `a = a + b`
                let lhs = Box::new(self.lower_expr(&assign_op_expr.target));
                let rhs_left = Box::new(self.lower_expr(&assign_op_expr.target));
                let rhs_right = Box::new(self.lower_expr(&assign_op_expr.value));
                let bin_op = self.lower_assign_op(&assign_op_expr.op.node);
                let rhs = Box::new(HirNode::new(
                    hir::Expr::Binary(hir::BinaryExpr {
                        lhs: rhs_left,
                        op: bin_op,
                        rhs: rhs_right,
                    }),
                    expr.span,
                ));
                hir::Expr::Assign(hir::AssignExpr { lhs, rhs })
            }
            ast::Expr::FieldAccess(field_access_expr) => {
                let base = Box::new(self.lower_expr(&field_access_expr.target));
                let field = field_access_expr.field.clone().into();
                hir::Expr::Field(hir::FieldExpr { base, field })
            }
            ast::Expr::Index(index_expr) => {
                let base = Box::new(self.lower_expr(&index_expr.target));
                let index = Box::new(self.lower_expr(&index_expr.index));
                hir::Expr::Index(hir::IndexExpr { base, index })
            }
            ast::Expr::Path(path_expr) => {
                let path = self.lower_path(&path_expr.path);
                hir::Expr::Path(path)
            }
            ast::Expr::AddrOf(addr_of_expr) => {
                let inner = Box::new(self.lower_expr(&addr_of_expr.expr));
                hir::Expr::AddrOf(inner)
            }
            ast::Expr::Break(break_expr) => {
                let value = break_expr.expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
                hir::Expr::Break(value)
            }
            ast::Expr::Continue => hir::Expr::Continue,
            ast::Expr::Literal(lit_expr) => {
                let lit = self.lower_literal(lit_expr);
                hir::Expr::Literal(lit)
            }
            ast::Expr::Binary(binary_expr) => {
                let lhs = Box::new(self.lower_expr(&binary_expr.left));
                let op = self.lower_bin_op(&binary_expr.operator.node);
                let rhs = Box::new(self.lower_expr(&binary_expr.right));
                hir::Expr::Binary(hir::BinaryExpr { lhs, op, rhs })
            }
            ast::Expr::Unary(unary_expr) => {
                let op = self.lower_un_op(&unary_expr.operator.node);
                let operand = Box::new(self.lower_expr(&unary_expr.operand));
                hir::Expr::Unary(hir::UnaryExpr { op, operand })
            }
            ast::Expr::If(if_expr) => {
                let condition = Box::new(self.lower_expr(&if_expr.condition));
                let then_branch = self.lower_block_expr(&if_expr.then_branch);
                let else_branch = if_expr.else_branch.as_ref().map(|e| self.lower_block_expr(e));
                hir::Expr::If(hir::IfExpr {
                    condition,
                    then_branch,
                    else_branch,
                })
            }
            ast::Expr::Block(block_expr) => {
                let block = self.lower_block_expr(&AstNode::with_id(block_expr.clone(), expr.span, expr.ast_id));
                hir::Expr::Block(block)
            }
            ast::Expr::Match(match_expr) => {
                let scrutinee = Box::new(self.lower_expr(&match_expr.value));
                let arms = match_expr.arms.iter().map(|arm| self.lower_match_arm(arm)).collect();
                hir::Expr::Match(hir::MatchExpr { scrutinee, arms })
            }
            ast::Expr::Let(let_expr) => {
                let pattern = self.lower_pattern(&let_expr.pattern);
                let init = Box::new(self.lower_expr(&let_expr.value));
                hir::Expr::Let(hir::LetExpr { pattern, init })
            }
            ast::Expr::Paren(inner) => self.lower_expr(inner).node,
            ast::Expr::Err => hir::Expr::Err,
        };

        HirNode::new(hir_expr, expr.span)
    }

    fn lower_struct_expr_field(&mut self, field: &AstNode<ast::StructExprField>) -> HirNode<hir::StructExprField> {
        let ident = field.node.ident.clone().into();
        let expr = Box::new(self.lower_expr(&field.node.expr));
        HirNode::new(hir::StructExprField { ident, expr }, field.span)
    }

    fn lower_match_arm(&mut self, arm: &AstNode<ast::MatchArm>) -> HirNode<hir::MatchArm> {
        let pattern = self.lower_pattern(&arm.node.pattern);
        let body = Box::new(self.lower_expr(&arm.node.body));
        HirNode::new(hir::MatchArm { pattern, body }, arm.span)
    }

    fn lower_literal(&self, lit: &ast::LiteralExpr) -> hir::Literal {
        match lit {
            ast::LiteralExpr::Bool(b) => hir::Literal::Bool(*b),
            ast::LiteralExpr::I32(i) => hir::Literal::I32(*i),
            ast::LiteralExpr::U32(u) => hir::Literal::U32(*u),
            ast::LiteralExpr::F64(f) => hir::Literal::F64(*f),
            ast::LiteralExpr::Str(s) => hir::Literal::Str(s.clone()),
            ast::LiteralExpr::Unit => hir::Literal::Unit,
        }
    }

    fn lower_bin_op(&self, op: &ast::BinOp) -> hir::BinOp {
        match op {
            ast::BinOp::Add => hir::BinOp::Add,
            ast::BinOp::Sub => hir::BinOp::Sub,
            ast::BinOp::Mul => hir::BinOp::Mul,
            ast::BinOp::Div => hir::BinOp::Div,
            ast::BinOp::Rem => hir::BinOp::Rem,
            ast::BinOp::And => hir::BinOp::And,
            ast::BinOp::Or => hir::BinOp::Or,
            ast::BinOp::EqEq => hir::BinOp::Eq,
            ast::BinOp::Less => hir::BinOp::Lt,
            ast::BinOp::LessEq => hir::BinOp::Le,
            ast::BinOp::Greater => hir::BinOp::Gt,
            ast::BinOp::GreaterEq => hir::BinOp::Ge,
            ast::BinOp::NotEq => hir::BinOp::Ne,
        }
    }

    fn lower_un_op(&self, op: &ast::UnOp) -> hir::UnOp {
        match op {
            ast::UnOp::Deref => hir::UnOp::Deref,
            ast::UnOp::Not => hir::UnOp::Not,
            ast::UnOp::Neg => hir::UnOp::Neg,
        }
    }

    fn lower_assign_op(&self, op: &ast::AssignOp) -> hir::BinOp {
        match op {
            ast::AssignOp::AddAssign => hir::BinOp::Add,
            ast::AssignOp::SubAssign => hir::BinOp::Sub,
            ast::AssignOp::MulAssign => hir::BinOp::Mul,
            ast::AssignOp::DivAssign => hir::BinOp::Div,
            ast::AssignOp::RemAssign => hir::BinOp::Rem,
        }
    }

    fn lower_block_expr(&mut self, block: &AstNode<ast::BlockExpr>) -> HirNode<hir::BlockExpr> {
        let stmts = block
            .node
            .stmts
            .iter()
            .filter(|stmt| !matches!(&stmt.node, ast::Stmt::Item(item) if matches!(item.node, ast::Item::Use(_))))
            .map(|stmt| self.lower_stmt(stmt))
            .collect();
        HirNode::new(hir::BlockExpr { stmts }, block.span)
    }

    fn lower_pattern(&mut self, pattern: &AstNode<ast::Pattern>) -> HirNode<hir::Pattern> {
        let pat = match &pattern.node {
            ast::Pattern::Wildcard => hir::Pattern::Wildcard,
            ast::Pattern::Or(patterns) => {
                hir::Pattern::Or(patterns.iter().map(|pat| self.lower_pattern(pat)).collect())
            }
            ast::Pattern::Path(path) => hir::Pattern::Path(self.lower_path(path)),
            ast::Pattern::Struct(path, struct_fields) => {
                let path = self.lower_path(path);
                let struct_fields = struct_fields
                    .iter()
                    .map(|pat| self.lower_pattern_struct_field(pat))
                    .collect();
                hir::Pattern::Struct(path, struct_fields)
            }
            ast::Pattern::TupleStruct(path, patterns) => {
                let path = self.lower_path(path);
                let patterns = patterns.iter().map(|pattern| self.lower_pattern(pattern)).collect();
                hir::Pattern::TupleStruct(path, patterns)
            }
            ast::Pattern::Tuple(patterns) => {
                let patterns = patterns.iter().map(|pattern| self.lower_pattern(pattern)).collect();
                hir::Pattern::Tuple(patterns)
            }
            ast::Pattern::Expr(expr) => hir::Pattern::Expr(Box::new(self.lower_expr(expr))),
            ast::Pattern::Paren(pattern) => self.lower_pattern(pattern).node,
        };

        HirNode::new(pat, pattern.span)
    }
    fn lower_pattern_struct_field(
        &mut self,
        struct_field: &AstNode<ast::PatternStructField>,
    ) -> HirNode<hir::PatternStructField> {
        let ident = struct_field.node.ident.clone().into();
        let pattern = self.lower_pattern(&struct_field.node.pattern);

        HirNode::new(hir::PatternStructField { ident, pattern }, struct_field.span)
    }

    fn lower_variant_data(&mut self, data: &AstNode<ast::VariantData>) -> HirNode<hir::VariantData> {
        let hir_data = match &data.node {
            ast::VariantData::Unit => hir::VariantData::Unit,
            ast::VariantData::Struct { fields } => {
                let fields = fields.iter().map(|f| self.lower_struct_field(f)).collect();
                hir::VariantData::Struct { fields }
            }
            ast::VariantData::Tuple { types } => {
                let types = types.iter().map(|t| self.lower_type(t)).collect();
                hir::VariantData::Tuple { types }
            }
        };
        HirNode::new(hir_data, data.span)
    }

    fn lower_struct_field(&mut self, field: &AstNode<ast::StructFieldDef>) -> HirNode<hir::StructField> {
        let def_id = *self.defs.ast_to_def.get(&field.ast_id).unwrap();
        let ident = field.node.ident.clone().into();
        let ty = self.lower_type(&field.node.type_annotation);
        HirNode::new(hir::StructField { def_id, ident, ty }, field.span)
    }

    fn lower_enum_variant(&mut self, variant: &AstNode<ast::EnumVariant>) -> HirNode<hir::EnumVariant> {
        let def_id = *self.defs.ast_to_def.get(&variant.ast_id).unwrap();
        let ident = variant.node.ident.clone().into();
        let data = self.lower_variant_data(&variant.node.data);
        HirNode::new(hir::EnumVariant { def_id, ident, data }, variant.span)
    }

    fn lower_associated_item(&mut self, item: &AstNode<ast::AssociatedItem>) -> HirNode<hir::AssociatedItem> {
        let hir_item = match &item.node {
            ast::AssociatedItem::Fn(sig, body) => {
                let sig = self.lower_fn_sig(sig);
                let body = body.as_ref().map(|b| self.lower_block_expr(b));
                hir::AssociatedItem::Fn(sig, body)
            }
            ast::AssociatedItem::Type(ty_alias) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();
                let ident = ty_alias.node.ident.clone().into();
                let generics = ty_alias
                    .node
                    .generics
                    .iter()
                    .map(|g| self.lower_generic_param(g))
                    .collect();
                let ty = ty_alias.node.ty.as_ref().map(|t| self.lower_type(t));
                hir::AssociatedItem::Type(HirNode::new(
                    hir::TyAlias {
                        def_id,
                        ident,
                        generics,
                        ty,
                    },
                    ty_alias.span,
                ))
            }
        };
        HirNode::new(hir_item, item.span)
    }
}
