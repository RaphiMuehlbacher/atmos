use crate::parser::ast::{
    AssociatedItem, BlockExpr, CallExpr, ConstDecl, Crate, EnumDecl, Expr, ExternFnDecl, FnDecl, FnSig, GenericArg,
    GenericParam, Ident, ImplDecl, Item, LetStmt, MatchArm, MethodCallExpr, Param, Path, PathSegment, Pattern, Stmt,
    StructDecl, StructExpr, StructExprField, TraitDecl, Ty, TyAliasDecl, VariantData,
};

macro_rules! visit_list {
    ($visitor: expr, $method: ident, $list: expr ) => {
        for elem in $list {
            $visitor.$method(&elem.node);
        }
    };
}

macro_rules! visit_opt {
    ($visitor: expr, $method: ident, $opt: expr ) => {
        if let Some(elem) = $opt {
            $visitor.$method(&elem.node);
        }
    };
}

pub trait Visitor: Sized {
    fn visit_crate(&mut self, krate: &Crate) {
        walk_crate(self, krate);
    }

    fn visit_ident(&mut self, _ident: &Ident) {}

    fn visit_item(&mut self, item: &Item) {
        walk_item(self, item);
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    fn visit_let_stmt(&mut self, let_stmt: &LetStmt) {
        walk_let_stmt(self, let_stmt);
    }

    fn visit_fn_sig(&mut self, fn_sig: &FnSig) {
        walk_fn_sig(self, fn_sig);
    }

    fn visit_block(&mut self, block: &BlockExpr) {
        walk_block(self, block);
    }

    fn visit_generic_param(&mut self, generic_param: &GenericParam) {
        walk_generic_param(self, generic_param);
    }

    fn visit_param(&mut self, param: &Param) {
        walk_param(self, param);
    }

    fn visit_type(&mut self, ty: &Ty) {
        walk_type(self, ty);
    }

    fn visit_pattern(&mut self, pattern: &Pattern) {
        walk_pattern(self, pattern);
    }

    fn visit_variant_data(&mut self, variant_data: &VariantData) {
        walk_variant_data(self, variant_data);
    }

    fn visit_assoc_item(&mut self, assoc_item: &AssociatedItem) {
        walk_assoc_item(self, assoc_item);
    }

    fn visit_path(&mut self, path: &Path) {
        walk_path(self, path);
    }

    fn visit_path_segment(&mut self, path_segment: &PathSegment) {
        walk_path_segment(self, path_segment);
    }

    fn visit_generic_arg(&mut self, generic_arg: &GenericArg) {
        walk_generic_arg(self, generic_arg);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }

    fn visit_match_arm(&mut self, arm: &MatchArm) {
        walk_match_arm(self, arm);
    }

    fn visit_struct_expr_field(&mut self, struct_expr_field: &StructExprField) {
        walk_struct_expr_field(self, struct_expr_field);
    }
}

pub fn walk_crate(visitor: &mut impl Visitor, krate: &Crate) {
    visit_list!(visitor, visit_item, &krate.items);
}

pub fn walk_item(visitor: &mut impl Visitor, item: &Item) {
    match item {
        Item::Fn(FnDecl { sig, body }) => {
            visitor.visit_fn_sig(&sig.node);
            visitor.visit_block(&body.node);
        }
        Item::Struct(StructDecl { ident, generics, data }) => {
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            visitor.visit_variant_data(&data.node);
        }
        Item::Enum(EnumDecl {
            ident,
            generics,
            variants,
        }) => {
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            for variant in variants {
                visitor.visit_ident(&variant.node.ident.node);
                visitor.visit_variant_data(&variant.node.data.node)
            }
        }
        Item::Trait(TraitDecl { ident, generics, items }) => {
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            visit_list!(visitor, visit_assoc_item, items);
        }
        Item::Impl(ImplDecl {
            generics,
            self_ty,
            for_trait,
            items,
        }) => {
            visit_list!(visitor, visit_generic_param, generics);
            visitor.visit_type(&self_ty.node);
            visit_opt!(visitor, visit_path, for_trait);
            visit_list!(visitor, visit_assoc_item, items);
        }
        Item::ExternFn(ExternFnDecl { sig }) => {
            visitor.visit_fn_sig(&sig.node);
        }
        Item::Const(ConstDecl {
            ident,
            generics,
            type_annotation,
            expr,
        }) => {
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, type_annotation);
            visitor.visit_expr(&expr.node);
        }
        Item::Use(use_item) => {
            visitor.visit_path(&use_item.path.node);
        }
        Item::TyAlias(ty_alias) => {
            let TyAliasDecl { ident, generics, ty } = &ty_alias;
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, ty);
        }
        Item::Err => {}
    }
}

pub fn walk_fn_sig(visitor: &mut impl Visitor, fn_sig: &FnSig) {
    visitor.visit_ident(&fn_sig.ident.node);
    visit_list!(visitor, visit_generic_param, &fn_sig.generics);
    visit_list!(visitor, visit_param, &fn_sig.params);
    visit_opt!(visitor, visit_type, &fn_sig.return_ty);
}

pub fn walk_variant_data(visitor: &mut impl Visitor, variant_data: &VariantData) {
    match variant_data {
        VariantData::Unit => {}
        VariantData::Struct { fields } => {
            for field in fields {
                visitor.visit_ident(&field.node.ident.node);
                visitor.visit_type(&field.node.type_annotation.node);
            }
        }
        VariantData::Tuple { types } => visit_list!(visitor, visit_type, types),
    }
}

pub fn walk_assoc_item(visitor: &mut impl Visitor, assoc_item: &AssociatedItem) {
    match assoc_item {
        AssociatedItem::Fn(sig, block) => {
            visitor.visit_fn_sig(&sig.node);
            visit_opt!(visitor, visit_block, block);
        }
        AssociatedItem::Type(ty_alias) => {
            let TyAliasDecl { ident, generics, ty } = &ty_alias.node;
            visitor.visit_ident(&ident.node);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, ty);
        }
    }
}

pub fn walk_path(visitor: &mut impl Visitor, path: &Path) {
    visit_list!(visitor, visit_path_segment, &path.segments);
}

pub fn walk_path_segment(visitor: &mut impl Visitor, path_segment: &PathSegment) {
    visitor.visit_ident(&path_segment.ident.node);
    visit_list!(visitor, visit_generic_arg, &path_segment.args);
}

pub fn walk_generic_arg(visitor: &mut impl Visitor, generic_arg: &GenericArg) {
    match generic_arg {
        GenericArg::Type(ty) => visitor.visit_type(&ty.node),
        GenericArg::Const(expr) => visitor.visit_expr(&expr.node),
    }
}

pub fn walk_expr(visitor: &mut impl Visitor, expr: &Expr) {
    match expr {
        Expr::Array(array_expr) => visit_list!(visitor, visit_expr, &array_expr.expressions),
        Expr::Struct(StructExpr { name, fields }) => {
            visitor.visit_path(&name.node);
            visit_list!(visitor, visit_struct_expr_field, fields);
        }
        Expr::Call(CallExpr { callee, arguments }) => {
            visitor.visit_expr(&callee.node);
            visit_list!(visitor, visit_expr, arguments);
        }
        Expr::MethodCall(MethodCallExpr {
            name,
            receiver,
            arguments,
        }) => {
            visitor.visit_path_segment(&name.node);
            visitor.visit_expr(&receiver.node);
            visit_list!(visitor, visit_expr, arguments);
        }
        Expr::Tuple(tuple_expr) => visit_list!(visitor, visit_expr, &tuple_expr.expressions),
        Expr::Cast(cast_expr) => {
            visitor.visit_expr(&cast_expr.expr.node);
            visitor.visit_type(&cast_expr.ty.node);
        }
        Expr::Return(return_expr) => visit_opt!(visitor, visit_expr, &return_expr.value),
        Expr::While(while_expr) => {
            visitor.visit_expr(&while_expr.condition.node);
            visitor.visit_block(&while_expr.body.node);
        }
        Expr::Loop(loop_expr) => visitor.visit_block(&loop_expr.body.node),
        Expr::For(for_expr) => {
            visitor.visit_pattern(&for_expr.pattern.node);
            visitor.visit_expr(&for_expr.iterator.node);
            visitor.visit_block(&for_expr.body.node);
        }
        Expr::Assign(assign_expr) => {
            visitor.visit_expr(&assign_expr.target.node);
            visitor.visit_expr(&assign_expr.value.node);
        }
        Expr::AssignOp(assign_op_expr) => {
            visitor.visit_expr(&assign_op_expr.target.node);
            visitor.visit_expr(&assign_op_expr.value.node);
        }
        Expr::FieldAccess(field_access_expr) => {
            visitor.visit_expr(&field_access_expr.target.node);
            visitor.visit_ident(&field_access_expr.field.node);
        }
        Expr::Index(index_expr) => {
            visitor.visit_expr(&index_expr.target.node);
            visitor.visit_expr(&index_expr.index.node);
        }
        Expr::Path(path_expr) => visitor.visit_path(&path_expr.path.node),
        Expr::AddrOf(addr_of_expr) => visitor.visit_expr(&addr_of_expr.expr.node),
        Expr::Break(break_expr) => visit_opt!(visitor, visit_expr, &break_expr.expr),
        Expr::Continue => {}
        Expr::Literal(literal_expr) => {}
        Expr::Binary(binary_expr) => {
            visitor.visit_expr(&binary_expr.left.node);
            visitor.visit_expr(&binary_expr.right.node);
        }
        Expr::Unary(unary_expr) => visitor.visit_expr(&unary_expr.operand.node),
        Expr::If(if_expr) => {
            visitor.visit_expr(&if_expr.condition.node);
            visitor.visit_block(&if_expr.then_branch.node);
            visit_opt!(visitor, visit_block, &if_expr.else_branch);
        }
        Expr::Block(block_expr) => visitor.visit_block(block_expr),
        Expr::Match(match_expr) => {
            visitor.visit_expr(&match_expr.value.node);
            visit_list!(visitor, visit_match_arm, &match_expr.arms);
        }
        Expr::Let(let_expr) => {
            visitor.visit_pattern(&let_expr.pattern.node);
            visitor.visit_expr(&let_expr.value.node);
        }
        Expr::Paren(paren_expr) => visitor.visit_expr(&paren_expr.node),
        Expr::Err => {}
    }
}

pub fn walk_struct_expr_field(visitor: &mut impl Visitor, struct_expr_field: &StructExprField) {
    visitor.visit_ident(&struct_expr_field.ident.node);
    visitor.visit_expr(&struct_expr_field.expr.node);
}

pub fn walk_stmt(visitor: &mut impl Visitor, stmt: &Stmt) {
    match stmt {
        Stmt::Item(item) => visitor.visit_item(&item.node),
        Stmt::Let(let_stmt) => visitor.visit_let_stmt(let_stmt),
        Stmt::Expr(expr) => visitor.visit_expr(&expr.node),
        Stmt::Semi(expr) => visitor.visit_expr(&expr.node),
        Stmt::Err => {}
    }
}

pub fn walk_let_stmt(visitor: &mut impl Visitor, let_stmt: &LetStmt) {
    visitor.visit_pattern(&let_stmt.pat.node);
    visit_opt!(visitor, visit_type, &let_stmt.type_annotation);
    visit_opt!(visitor, visit_expr, &let_stmt.expr);
}

pub fn walk_block(visitor: &mut impl Visitor, block: &BlockExpr) {
    visit_list!(visitor, visit_stmt, &block.stmts);
}

pub fn walk_generic_param(visitor: &mut impl Visitor, generic_param: &GenericParam) {
    visitor.visit_ident(&generic_param.ident.node);
    visit_list!(visitor, visit_path, &generic_param.bounds);
}

pub fn walk_param(visitor: &mut impl Visitor, param: &Param) {
    visitor.visit_pattern(&param.pattern.node);
    visitor.visit_type(&param.type_annotation.node);
}

pub fn walk_type(visitor: &mut impl Visitor, ty: &Ty) {
    match ty {
        Ty::Path(path) => visitor.visit_path(&path.node),
        Ty::Array(ty, expr) => {
            visitor.visit_type(&ty.node);
            visitor.visit_expr(&expr.node);
        }
        Ty::Ptr(ty) => visitor.visit_type(&ty.node),
        Ty::Fn(params, return_ty) => {
            visit_list!(visitor, visit_type, params);
            visit_opt!(visitor, visit_type, return_ty.as_ref());
        }
        Ty::Tuple(types) => visit_list!(visitor, visit_type, types),
        Ty::Paren(ty) => visitor.visit_type(&ty.node),
    }
}

pub fn walk_pattern(visitor: &mut impl Visitor, pattern: &Pattern) {
    match pattern {
        Pattern::Wildcard => {}
        Pattern::Or(patterns) => visit_list!(visitor, visit_pattern, patterns),
        Pattern::Path(path) => visitor.visit_path(&path.node),
        Pattern::Struct(path, fields) => {
            visitor.visit_path(&path.node);
            for field in fields {
                visitor.visit_ident(&field.node.ident.node);
                visitor.visit_pattern(&field.node.pattern.node);
            }
        }
        Pattern::TupleStruct(path, patterns) => {
            visitor.visit_path(&path.node);
            visit_list!(visitor, visit_pattern, patterns);
        }
        Pattern::Tuple(patterns) => visit_list!(visitor, visit_pattern, patterns),
        Pattern::Expr(expr) => visitor.visit_expr(&expr.node),
        Pattern::Paren(pattern) => visitor.visit_pattern(&pattern.node),
    }
}

pub fn walk_match_arm(visitor: &mut impl Visitor, arm: &MatchArm) {
    visitor.visit_pattern(&arm.pattern.node);
    visitor.visit_expr(&arm.body.node);
}
