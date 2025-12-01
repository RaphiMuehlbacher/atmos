use crate::parser::ast::{
    AssociatedItem, AstNode, BlockExpr, CallExpr, ConstDecl, Crate, EnumDecl, EnumVariant, Expr, ExternFnDecl, FnDecl,
    FnSig, GenericArg, GenericParam, Ident, ImplDecl, Item, LetStmt, MatchArm, MethodCallExpr, ModDecl, Param, Path,
    PathSegment, Pattern, Stmt, StructDecl, StructExpr, StructExprField, StructFieldDef, TraitDecl, Ty, TyAliasDecl,
    VariantData,
};

macro_rules! visit_list {
    ($visitor: expr, $method: ident, $list: expr ) => {
        for elem in $list {
            $visitor.$method(elem);
        }
    };
}

macro_rules! visit_opt {
    ($visitor: expr, $method: ident, $opt: expr ) => {
        if let Some(elem) = $opt {
            $visitor.$method(elem);
        }
    };
}

pub trait Visitor: Sized {
    fn visit_crate(&mut self, krate: &Crate) {
        walk_crate(self, krate);
    }

    fn visit_ident(&mut self, _ident: &AstNode<Ident>) {}

    fn visit_item(&mut self, item: &AstNode<Item>) {
        walk_item(self, item);
    }

    fn visit_stmt(&mut self, stmt: &AstNode<Stmt>) {
        walk_stmt(self, stmt);
    }

    fn visit_let_stmt(&mut self, let_stmt: &AstNode<LetStmt>) {
        walk_let_stmt(self, let_stmt);
    }

    fn visit_fn_sig(&mut self, fn_sig: &AstNode<FnSig>) {
        walk_fn_sig(self, fn_sig);
    }

    fn visit_block(&mut self, block: &AstNode<BlockExpr>) {
        walk_block(self, block);
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        walk_generic_param(self, generic_param);
    }

    fn visit_param(&mut self, param: &AstNode<Param>) {
        walk_param(self, param);
    }

    fn visit_type(&mut self, ty: &AstNode<Ty>) {
        walk_type(self, ty);
    }

    fn visit_pattern(&mut self, pattern: &AstNode<Pattern>) {
        walk_pattern(self, pattern);
    }

    fn visit_variant_data(&mut self, variant_data: &AstNode<VariantData>) {
        walk_variant_data(self, variant_data);
    }

    fn visit_struct_field_def(&mut self, struct_field_def: &AstNode<StructFieldDef>) {
        walk_struct_field_def(self, struct_field_def)
    }
    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        walk_enum_variant(self, enum_variant);
    }

    fn visit_assoc_item(&mut self, assoc_item: &AstNode<AssociatedItem>) {
        walk_assoc_item(self, assoc_item);
    }

    fn visit_path(&mut self, path: &AstNode<Path>) {
        walk_path(self, path);
    }

    fn visit_path_segment(&mut self, path_segment: &AstNode<PathSegment>) {
        walk_path_segment(self, path_segment);
    }

    fn visit_generic_arg(&mut self, generic_arg: &AstNode<GenericArg>) {
        walk_generic_arg(self, generic_arg);
    }

    fn visit_expr(&mut self, expr: &AstNode<Expr>) {
        walk_expr(self, expr);
    }

    fn visit_match_arm(&mut self, arm: &AstNode<MatchArm>) {
        walk_match_arm(self, arm);
    }

    fn visit_struct_expr_field(&mut self, struct_expr_field: &AstNode<StructExprField>) {
        walk_struct_expr_field(self, struct_expr_field);
    }
}

pub fn walk_crate(visitor: &mut impl Visitor, krate: &Crate) {
    visit_list!(visitor, visit_item, &krate.items);
}

pub fn walk_item(visitor: &mut impl Visitor, item: &AstNode<Item>) {
    match &item.node {
        Item::Fn(FnDecl { sig, body }) => {
            visitor.visit_fn_sig(sig);
            visitor.visit_block(body);
        }
        Item::Struct(StructDecl { ident, generics, data }) => {
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visitor.visit_variant_data(data);
        }
        Item::Enum(EnumDecl {
            ident,
            generics,
            variants,
        }) => {
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visit_list!(visitor, visit_enum_variant, variants);
        }
        Item::Trait(TraitDecl { ident, generics, items }) => {
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visit_list!(visitor, visit_assoc_item, items);
        }
        Item::Mod(ModDecl { ident, items }) => {
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_item, items);
        }
        Item::Impl(ImplDecl {
            generics,
            self_ty,
            for_trait,
            items,
        }) => {
            visit_list!(visitor, visit_generic_param, generics);
            visitor.visit_type(self_ty);
            visit_opt!(visitor, visit_path, for_trait);
            visit_list!(visitor, visit_assoc_item, items);
        }
        Item::ExternFn(ExternFnDecl { sig }) => {
            visitor.visit_fn_sig(sig);
        }
        Item::Const(ConstDecl {
            ident,
            generics,
            type_annotation,
            expr,
        }) => {
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, type_annotation);
            visitor.visit_expr(expr);
        }
        Item::Use(use_item) => {
            visitor.visit_path(&use_item.path);
        }
        Item::TyAlias(ty_alias) => {
            let TyAliasDecl { ident, generics, ty } = &ty_alias;
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, ty);
        }
        Item::Err => {}
    }
}

pub fn walk_fn_sig(visitor: &mut impl Visitor, fn_sig: &AstNode<FnSig>) {
    visitor.visit_ident(&fn_sig.node.ident);
    visit_list!(visitor, visit_generic_param, &fn_sig.node.generics);
    visit_list!(visitor, visit_param, &fn_sig.node.params);
    visit_opt!(visitor, visit_type, &fn_sig.node.return_ty);
}

pub fn walk_variant_data(visitor: &mut impl Visitor, variant_data: &AstNode<VariantData>) {
    match &variant_data.node {
        VariantData::Unit => {}
        VariantData::Struct { fields } => visit_list!(visitor, visit_struct_field_def, fields),
        VariantData::Tuple { types } => visit_list!(visitor, visit_type, types),
    }
}

pub fn walk_enum_variant(visitor: &mut impl Visitor, enum_variant: &AstNode<EnumVariant>) {
    visitor.visit_ident(&enum_variant.node.ident);
    visitor.visit_variant_data(&enum_variant.node.data)
}

pub fn walk_struct_field_def(visitor: &mut impl Visitor, struct_field_def: &AstNode<StructFieldDef>) {
    visitor.visit_ident(&struct_field_def.node.ident);
    visitor.visit_type(&struct_field_def.node.type_annotation);
}

pub fn walk_assoc_item(visitor: &mut impl Visitor, assoc_item: &AstNode<AssociatedItem>) {
    match &assoc_item.node {
        AssociatedItem::Fn(sig, block) => {
            visitor.visit_fn_sig(sig);
            visit_opt!(visitor, visit_block, block);
        }
        AssociatedItem::Type(ty_alias) => {
            let TyAliasDecl { ident, generics, ty } = &ty_alias.node;
            visitor.visit_ident(ident);
            visit_list!(visitor, visit_generic_param, generics);
            visit_opt!(visitor, visit_type, ty);
        }
    }
}

pub fn walk_path(visitor: &mut impl Visitor, path: &AstNode<Path>) {
    visit_list!(visitor, visit_path_segment, &path.node.segments);
}

pub fn walk_path_segment(visitor: &mut impl Visitor, path_segment: &AstNode<PathSegment>) {
    visitor.visit_ident(&path_segment.node.ident);
    visit_list!(visitor, visit_generic_arg, &path_segment.node.args);
}

pub fn walk_generic_arg(visitor: &mut impl Visitor, generic_arg: &AstNode<GenericArg>) {
    match &generic_arg.node {
        GenericArg::Type(ty) => visitor.visit_type(ty),
        GenericArg::Const(expr) => visitor.visit_expr(expr),
    }
}

pub fn walk_expr(visitor: &mut impl Visitor, expr: &AstNode<Expr>) {
    match &expr.node {
        Expr::Array(array_expr) => visit_list!(visitor, visit_expr, &array_expr.expressions),
        Expr::Struct(StructExpr { name, fields }) => {
            visitor.visit_path(name);
            visit_list!(visitor, visit_struct_expr_field, fields);
        }
        Expr::Call(CallExpr { callee, arguments }) => {
            visitor.visit_expr(callee);
            visit_list!(visitor, visit_expr, arguments);
        }
        Expr::MethodCall(MethodCallExpr {
            name,
            receiver,
            arguments,
        }) => {
            visitor.visit_path_segment(name);
            visitor.visit_expr(receiver);
            visit_list!(visitor, visit_expr, arguments);
        }
        Expr::Tuple(tuple_expr) => visit_list!(visitor, visit_expr, &tuple_expr.expressions),
        Expr::Cast(cast_expr) => {
            visitor.visit_expr(&cast_expr.expr);
            visitor.visit_type(&cast_expr.ty);
        }
        Expr::Return(return_expr) => visit_opt!(visitor, visit_expr, &return_expr.value),
        Expr::While(while_expr) => {
            visitor.visit_expr(&while_expr.condition);
            visitor.visit_block(&while_expr.body);
        }
        Expr::Loop(loop_expr) => visitor.visit_block(&loop_expr.body),
        Expr::For(for_expr) => {
            visitor.visit_pattern(&for_expr.pattern);
            visitor.visit_expr(&for_expr.iterator);
            visitor.visit_block(&for_expr.body);
        }
        Expr::Assign(assign_expr) => {
            visitor.visit_expr(&assign_expr.target);
            visitor.visit_expr(&assign_expr.value);
        }
        Expr::AssignOp(assign_op_expr) => {
            visitor.visit_expr(&assign_op_expr.target);
            visitor.visit_expr(&assign_op_expr.value);
        }
        Expr::FieldAccess(field_access_expr) => {
            visitor.visit_expr(&field_access_expr.target);
            visitor.visit_ident(&field_access_expr.field);
        }
        Expr::Index(index_expr) => {
            visitor.visit_expr(&index_expr.target);
            visitor.visit_expr(&index_expr.index);
        }
        Expr::Path(path_expr) => visitor.visit_path(&path_expr.path),
        Expr::AddrOf(addr_of_expr) => visitor.visit_expr(&addr_of_expr.expr),
        Expr::Break(break_expr) => visit_opt!(visitor, visit_expr, &break_expr.expr),
        Expr::Continue => {}
        Expr::Literal(_) => {}
        Expr::Binary(binary_expr) => {
            visitor.visit_expr(&binary_expr.left);
            visitor.visit_expr(&binary_expr.right);
        }
        Expr::Unary(unary_expr) => visitor.visit_expr(&unary_expr.operand),
        Expr::If(if_expr) => {
            visitor.visit_expr(&if_expr.condition);
            visitor.visit_block(&if_expr.then_branch);
            visit_opt!(visitor, visit_block, &if_expr.else_branch);
        }
        Expr::Block(block_expr) => {
            let block_node = AstNode::new(block_expr.clone(), expr.span);
            visitor.visit_block(&block_node);
        }
        Expr::Match(match_expr) => {
            visitor.visit_expr(&match_expr.value);
            visit_list!(visitor, visit_match_arm, &match_expr.arms);
        }
        Expr::Let(_let_expr) => {}
        Expr::Paren(paren_expr) => visitor.visit_expr(paren_expr),
        Expr::Err => {}
    }
}

pub fn walk_struct_expr_field(visitor: &mut impl Visitor, struct_expr_field: &AstNode<StructExprField>) {
    visitor.visit_ident(&struct_expr_field.node.ident);
    visitor.visit_expr(&struct_expr_field.node.expr);
}

pub fn walk_stmt(visitor: &mut impl Visitor, stmt: &AstNode<Stmt>) {
    match &stmt.node {
        Stmt::Item(item) => visitor.visit_item(item),
        Stmt::Let(let_stmt) => {
            let let_stmt_node = AstNode::new(let_stmt.clone(), stmt.span);
            visitor.visit_let_stmt(&let_stmt_node);
        }
        Stmt::Expr(expr) => visitor.visit_expr(expr),
        Stmt::Semi(expr) => visitor.visit_expr(expr),
        Stmt::Err => {}
    }
}

pub fn walk_let_stmt(visitor: &mut impl Visitor, let_stmt: &AstNode<LetStmt>) {
    visitor.visit_pattern(&let_stmt.node.pat);
    visit_opt!(visitor, visit_type, &let_stmt.node.type_annotation);
    visit_opt!(visitor, visit_expr, &let_stmt.node.expr);
}

pub fn walk_block(visitor: &mut impl Visitor, block: &AstNode<BlockExpr>) {
    visit_list!(visitor, visit_stmt, &block.node.stmts);
}

pub fn walk_generic_param(visitor: &mut impl Visitor, generic_param: &AstNode<GenericParam>) {
    visitor.visit_ident(&generic_param.node.ident);
    visit_list!(visitor, visit_path, &generic_param.node.bounds);
}

pub fn walk_param(visitor: &mut impl Visitor, param: &AstNode<Param>) {
    visitor.visit_pattern(&param.node.pattern);
    visitor.visit_type(&param.node.type_annotation);
}

pub fn walk_type(visitor: &mut impl Visitor, ty: &AstNode<Ty>) {
    match &ty.node {
        Ty::Path(path) => visitor.visit_path(path),
        Ty::Array(ty, expr) => {
            visitor.visit_type(ty);
            visitor.visit_expr(expr);
        }
        Ty::Ptr(ty) => visitor.visit_type(ty),
        Ty::Fn(params, return_ty) => {
            visit_list!(visitor, visit_type, params);
            visit_opt!(visitor, visit_type, return_ty.as_ref());
        }
        Ty::Tuple(types) => visit_list!(visitor, visit_type, types),
        Ty::Paren(ty) => visitor.visit_type(ty),
    }
}

pub fn walk_pattern(visitor: &mut impl Visitor, pattern: &AstNode<Pattern>) {
    match &pattern.node {
        Pattern::Wildcard => {}
        Pattern::Or(patterns) => visit_list!(visitor, visit_pattern, patterns),
        Pattern::Path(path) => visitor.visit_path(path),
        Pattern::Struct(path, fields) => {
            visitor.visit_path(path);
            for field in fields {
                visitor.visit_ident(&field.node.ident);
                visitor.visit_pattern(&field.node.pattern);
            }
        }
        Pattern::TupleStruct(path, patterns) => {
            visitor.visit_path(path);
            visit_list!(visitor, visit_pattern, patterns);
        }
        Pattern::Tuple(patterns) => visit_list!(visitor, visit_pattern, patterns),
        Pattern::Expr(expr) => visitor.visit_expr(expr),
        Pattern::Paren(pattern) => visitor.visit_pattern(pattern),
    }
}

pub fn walk_match_arm(visitor: &mut impl Visitor, arm: &AstNode<MatchArm>) {
    visitor.visit_pattern(&arm.node.pattern);
    visitor.visit_expr(&arm.node.body);
}
