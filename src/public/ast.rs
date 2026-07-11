use crate::parser::ast as ast_internal;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::span::Span;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AstId(pub usize);

impl From<ast_internal::AstId> for AstId {
    fn from(ast_id: ast_internal::AstId) -> Self {
        Self(ast_id.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstCrate {
    pub root_items: Vec<AstId>,
    pub nodes: HashMap<AstId, AstNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub id: AstId,
    pub span: Span,
    pub kind: String,
    pub label: String,
    pub children: Vec<AstId>,
}

impl AstCrate {
    pub fn from_krate(krate: &ast_internal::Crate) -> Self {
        let mut converter = AstConverter {
            nodes: HashMap::new(),
        };

        let root_items: Vec<AstId> = krate
            .items
            .iter()
            .map(|item| converter.convert_item(item))
            .collect();

        AstCrate {
            root_items,
            nodes: converter.nodes,
        }
    }
}

struct AstConverter {
    nodes: HashMap<AstId, AstNode>,
}

impl AstConverter {
    fn insert_node(
        &mut self,
        id: AstId,
        kind: &str,
        label: String,
        children: Vec<AstId>,
        span: impl Into<Span>,
    ) {
        self.nodes.insert(
            id,
            AstNode {
                id,
                span: span.into(),
                kind: kind.to_string(),
                label,
                children,
            },
        );
    }

    fn convert_item(&mut self, item: &ast_internal::AstNode<ast_internal::Item>) -> AstId {
        let id = item.ast_id.into();
        let (kind, label, children) = match &item.node {
            ast_internal::Item::Fn(f) => {
                let sig_id = self.convert_fn_sig(&f.sig);
                let body_id = self.convert_block_expr(&f.body);
                let name = f.sig.node.ident.node.name.clone();
                ("fn", format!("fn {name}"), vec![sig_id, body_id])
            }
            ast_internal::Item::Struct(s) => {
                let data_id = self.convert_variant_data(&s.data);
                let generics_ids: Vec<AstId> = s
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = s.ident.node.name.clone();
                ("struct", format!("struct {name}"), {
                    let mut v = generics_ids;
                    v.push(data_id);
                    v
                })
            }
            ast_internal::Item::Enum(e) => {
                let variant_ids: Vec<AstId> = e
                    .variants
                    .iter()
                    .map(|v| self.convert_enum_variant(v))
                    .collect();
                let generics_ids: Vec<AstId> = e
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = e.ident.node.name.clone();
                ("enum", format!("enum {name}"), {
                    let mut v = generics_ids;
                    v.extend(variant_ids);
                    v
                })
            }
            ast_internal::Item::Trait(t) => {
                let items_ids: Vec<AstId> = t
                    .items
                    .iter()
                    .map(|i| self.convert_associated_item(i))
                    .collect();
                let generics_ids: Vec<AstId> = t
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = t.ident.node.name.clone();
                ("trait", format!("trait {name}"), {
                    let mut v = generics_ids;
                    v.extend(items_ids);
                    v
                })
            }
            ast_internal::Item::Mod(m) => {
                let items_ids: Vec<AstId> = m.items.iter().map(|i| self.convert_item(i)).collect();
                let name = m.ident.node.name.clone();
                ("mod", format!("mod {name}"), items_ids)
            }
            ast_internal::Item::Impl(i) => {
                let self_ty_id = self.convert_ty(&i.self_ty);
                let trait_id = i.for_trait.as_ref().map(|t| self.convert_path(t));
                let items_ids: Vec<AstId> = i
                    .items
                    .iter()
                    .map(|a| self.convert_associated_item(a))
                    .collect();
                let mut children = Vec::new();
                if let Some(t) = trait_id {
                    children.push(t);
                }
                children.push(self_ty_id);
                children.extend(items_ids);
                let label = if i.for_trait.is_some() {
                    "impl ... for ...".to_string()
                } else {
                    "impl".to_string()
                };
                ("impl", label, children)
            }
            ast_internal::Item::ExternFn(f) => {
                let sig_id = self.convert_fn_sig(&f.sig);
                let name = f.sig.node.ident.node.name.clone();
                ("extern_fn", format!("extern fn {name}"), vec![sig_id])
            }
            ast_internal::Item::Const(c) => {
                let ty_id = self.convert_ty(&c.type_annotation);
                let expr_id = self.convert_expr(&c.expr);
                let name = c.ident.node.name.clone();
                ("const", format!("const {name}"), vec![ty_id, expr_id])
            }
            ast_internal::Item::Use(u) => {
                let path_id = self.convert_path(&u.path);
                ("use", "use".into(), vec![path_id])
            }
            ast_internal::Item::TyAlias(t) => {
                let ty_id = self.convert_ty(&t.ty);
                let generics_ids: Vec<AstId> = t
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = t.ident.node.name.clone();
                ("type_alias", format!("type {name}"), {
                    let mut v = generics_ids;
                    v.push(ty_id);
                    v
                })
            }
        };
        self.insert_node(id, kind, label, children, item.span);
        id
    }

    fn convert_fn_sig(&mut self, sig: &ast_internal::AstNode<ast_internal::FnSig>) -> AstId {
        let id = sig.ast_id.into();
        let name = sig.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&sig.node.ident);
        let generic_ids: Vec<AstId> = sig
            .node
            .generics
            .iter()
            .map(|g| self.convert_generic_param(g))
            .collect();
        let param_ids: Vec<AstId> = sig
            .node
            .params
            .iter()
            .map(|p| self.convert_param(p))
            .collect();
        let return_id = sig.node.return_ty.as_ref().map(|t| self.convert_ty(t));
        let mut children = vec![ident_id];
        children.extend(generic_ids);
        children.extend(param_ids);
        if let Some(r) = return_id {
            children.push(r);
        }
        self.insert_node(id, "fn_sig", name, children, sig.span);
        id
    }

    fn convert_param(&mut self, param: &ast_internal::AstNode<ast_internal::Param>) -> AstId {
        let id = param.ast_id.into();
        let pat_id = self.convert_pattern(&param.node.pattern);
        let ty_id = self.convert_ty(&param.node.type_annotation);
        self.insert_node(id, "param", "param".into(), vec![pat_id, ty_id], param.span);
        id
    }

    fn convert_generic_param(
        &mut self,
        gp: &ast_internal::AstNode<ast_internal::GenericParam>,
    ) -> AstId {
        let id = gp.ast_id.into();
        let name = gp.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&gp.node.ident);
        let bound_ids: Vec<AstId> = gp
            .node
            .bounds
            .iter()
            .map(|b| self.convert_path(b))
            .collect();
        let kind_child = match &gp.node.kind {
            ast_internal::GenericParamKind::Const(ty) => {
                vec![self.convert_ty(ty)]
            }
            ast_internal::GenericParamKind::Type => vec![],
        };
        let mut children = vec![ident_id];
        children.extend(bound_ids);
        children.extend(kind_child);
        self.insert_node(id, "generic_param", name, children, gp.span);
        id
    }

    fn convert_variant_data(
        &mut self,
        data: &ast_internal::AstNode<ast_internal::VariantData>,
    ) -> AstId {
        let id = data.ast_id.into();
        let (kind, label, children) = match &data.node {
            ast_internal::VariantData::Unit => ("variant_data_unit", "unit".into(), vec![]),
            ast_internal::VariantData::Struct { fields } => {
                let fids: Vec<AstId> = fields.iter().map(|f| self.convert_field_def(f)).collect();
                ("variant_data_struct", "struct".into(), fids)
            }
            ast_internal::VariantData::Tuple { fields } => {
                let fids: Vec<AstId> = fields.iter().map(|f| self.convert_field_def(f)).collect();
                ("variant_data_tuple", "tuple".into(), fids)
            }
        };
        self.insert_node(id, kind, label, children, data.span);
        id
    }

    fn convert_field_def(
        &mut self,
        field: &ast_internal::AstNode<ast_internal::FieldDef>,
    ) -> AstId {
        let id = field.ast_id.into();
        let name = field.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&field.node.ident);
        let ty_id = self.convert_ty(&field.node.type_annotation);
        self.insert_node(id, "field_def", name, vec![ident_id, ty_id], field.span);
        id
    }

    fn convert_enum_variant(
        &mut self,
        variant: &ast_internal::AstNode<ast_internal::EnumVariant>,
    ) -> AstId {
        let id = variant.ast_id.into();
        let name = variant.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&variant.node.ident);
        let data_id = self.convert_variant_data(&variant.node.data);
        self.insert_node(
            id,
            "enum_variant",
            name,
            vec![ident_id, data_id],
            variant.span,
        );
        id
    }

    fn convert_associated_item(
        &mut self,
        item: &ast_internal::AstNode<ast_internal::AssociatedItem>,
    ) -> AstId {
        let id = item.ast_id.into();
        let (kind, label, children) = match &item.node {
            ast_internal::AssociatedItem::Fn(sig, body) => {
                let sig_id = self.convert_fn_sig(sig);
                let body_id = body.as_ref().map(|b| self.convert_block_expr(b));
                let mut v = vec![sig_id];
                if let Some(b) = body_id {
                    v.push(b);
                }
                ("assoc_fn", "fn".into(), v)
            }
            ast_internal::AssociatedItem::Type(alias) => {
                let ident_id = self.convert_ident(&alias.node.ident);
                let ty_id = alias.node.ty.as_ref().map(|t| self.convert_ty(t));
                let generics_ids: Vec<AstId> = alias
                    .node
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let mut v = vec![ident_id];
                v.extend(generics_ids);
                if let Some(t) = ty_id {
                    v.push(t);
                }
                ("assoc_type", "type".into(), v)
            }
        };
        self.insert_node(id, kind, label, children, item.span);
        id
    }

    fn convert_stmt(&mut self, stmt: &ast_internal::AstNode<ast_internal::Stmt>) -> AstId {
        let id = stmt.ast_id.into();
        let (kind, label, children) = match &stmt.node {
            ast_internal::Stmt::Item(item) => {
                let child = self.convert_item(item);
                ("stmt_item", "item".into(), vec![child])
            }
            ast_internal::Stmt::Let(let_stmt) => {
                let pat_id = self.convert_pattern(&let_stmt.pat);
                let ty_id = let_stmt
                    .type_annotation
                    .as_ref()
                    .map(|t| self.convert_ty(t));
                let expr_id = let_stmt.expr.as_ref().map(|e| self.convert_expr(e));
                let mut children = vec![pat_id];
                if let Some(t) = ty_id {
                    children.push(t);
                }
                if let Some(e) = expr_id {
                    children.push(e);
                }
                ("let", "let".into(), children)
            }
            ast_internal::Stmt::Expr(expr) => {
                let child = self.convert_expr(expr);
                ("stmt_expr", "expr".into(), vec![child])
            }
            ast_internal::Stmt::Semi(expr) => {
                let child = self.convert_expr(expr);
                ("stmt_semi", "semi".into(), vec![child])
            }
            ast_internal::Stmt::Err => ("stmt_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, stmt.span);
        id
    }

    fn convert_expr(&mut self, expr: &ast_internal::AstNode<ast_internal::Expr>) -> AstId {
        let id = expr.ast_id.into();
        let (kind, label, children) = match &expr.node {
            ast_internal::Expr::Array(a) => {
                let kids: Vec<AstId> = a.expressions.iter().map(|e| self.convert_expr(e)).collect();
                ("array", "[]".into(), kids)
            }
            ast_internal::Expr::Struct(s) => {
                let path_id = self.convert_path(&s.name);
                let field_ids: Vec<AstId> = s
                    .fields
                    .iter()
                    .map(|f| self.convert_struct_expr_field(f))
                    .collect();
                let mut v = vec![path_id];
                v.extend(field_ids);
                ("struct_expr", "struct".into(), v)
            }
            ast_internal::Expr::Call(c) => {
                let callee_id = self.convert_expr(&c.callee);
                let arg_ids: Vec<AstId> =
                    c.arguments.iter().map(|a| self.convert_expr(a)).collect();
                let mut v = vec![callee_id];
                v.extend(arg_ids);
                ("call", "call".into(), v)
            }
            ast_internal::Expr::MethodCall(m) => {
                let receiver_id = self.convert_expr(&m.receiver);
                let name_id = self.convert_segment(&m.name);
                let arg_ids: Vec<AstId> =
                    m.arguments.iter().map(|a| self.convert_expr(a)).collect();
                let mut v = vec![receiver_id, name_id];
                v.extend(arg_ids);
                ("method_call", "method_call".into(), v)
            }
            ast_internal::Expr::Tuple(t) => {
                let kids: Vec<AstId> = t.expressions.iter().map(|e| self.convert_expr(e)).collect();
                ("tuple", "()".into(), kids)
            }
            ast_internal::Expr::Cast(c) => {
                let expr_id = self.convert_expr(&c.expr);
                let ty_id = self.convert_ty(&c.ty);
                ("cast", "as".into(), vec![expr_id, ty_id])
            }
            ast_internal::Expr::Return(r) => {
                let child = r.value.as_ref().map(|v| self.convert_expr(v));
                let mut v = Vec::new();
                if let Some(c) = child {
                    v.push(c);
                }
                ("return", "return".into(), v)
            }
            ast_internal::Expr::While(w) => {
                let cond_id = self.convert_expr(&w.condition);
                let body_id = self.convert_block_expr(&w.body);
                ("while", "while".into(), vec![cond_id, body_id])
            }
            ast_internal::Expr::Loop(l) => {
                let body_id = self.convert_block_expr(&l.body);
                ("loop", "loop".into(), vec![body_id])
            }
            ast_internal::Expr::For(f) => {
                let pat_id = self.convert_pattern(&f.pattern);
                let iter_id = self.convert_expr(&f.iterator);
                let body_id = self.convert_block_expr(&f.body);
                ("for", "for".into(), vec![pat_id, iter_id, body_id])
            }
            ast_internal::Expr::Assign(a) => {
                let target_id = self.convert_expr(&a.target);
                let value_id = self.convert_expr(&a.value);
                ("assign", "=".into(), vec![target_id, value_id])
            }
            ast_internal::Expr::AssignOp(a) => {
                let target_id = self.convert_expr(&a.target);
                let op_label = format!("{:?}", a.op.node);
                let value_id = self.convert_expr(&a.value);
                ("assign_op", op_label, vec![target_id, value_id])
            }
            ast_internal::Expr::FieldAccess(f) => {
                let target_id = self.convert_expr(&f.target);
                let field_id = self.convert_ident(&f.field);
                ("field_access", ".".into(), vec![target_id, field_id])
            }
            ast_internal::Expr::Index(i) => {
                let target_id = self.convert_expr(&i.target);
                let index_id = self.convert_expr(&i.index);
                ("index", "[]".into(), vec![target_id, index_id])
            }
            ast_internal::Expr::Path(p) => {
                let path_id = self.convert_path(&p.path);
                ("path", "path".into(), vec![path_id])
            }
            ast_internal::Expr::AddrOf(a) => {
                let inner_id = self.convert_expr(&a.expr);
                ("addr_of", "&".into(), vec![inner_id])
            }
            ast_internal::Expr::Break(b) => {
                let child = b.expr.as_ref().map(|e| self.convert_expr(e));
                let mut v = Vec::new();
                if let Some(c) = child {
                    v.push(c);
                }
                ("break", "break".into(), v)
            }
            ast_internal::Expr::Continue => ("continue", "continue".into(), vec![]),
            ast_internal::Expr::Literal(l) => {
                let label = match l {
                    ast_internal::LiteralExpr::Bool(b) => b.to_string(),
                    ast_internal::LiteralExpr::I32(i) => i.to_string(),
                    ast_internal::LiteralExpr::U32(u) => u.to_string(),
                    ast_internal::LiteralExpr::F64(f) => f.to_string(),
                    ast_internal::LiteralExpr::Str(s) => format!("\"{s}\""),
                    ast_internal::LiteralExpr::Unit => "()".into(),
                };
                ("literal", label, vec![])
            }
            ast_internal::Expr::Binary(b) => {
                let left_id = self.convert_expr(&b.left);
                let right_id = self.convert_expr(&b.right);
                let op_label = format!("{:?}", b.operator.node);
                ("binary", op_label, vec![left_id, right_id])
            }
            ast_internal::Expr::Unary(u) => {
                let operand_id = self.convert_expr(&u.operand);
                let op_label = format!("{:?}", u.operator.node);
                ("unary", op_label, vec![operand_id])
            }
            ast_internal::Expr::If(if_expr) => {
                let cond_id = self.convert_expr(&if_expr.condition);
                let then_id = self.convert_block_expr(&if_expr.then_branch);
                let else_id = if_expr
                    .else_branch
                    .as_ref()
                    .map(|e| self.convert_block_expr(e));
                let mut v = vec![cond_id, then_id];
                if let Some(e) = else_id {
                    v.push(e);
                }
                ("if", "if".into(), v)
            }
            ast_internal::Expr::Block(b) => {
                let stmt_ids: Vec<AstId> = b.stmts.iter().map(|s| self.convert_stmt(s)).collect();
                ("block", "{ }".into(), stmt_ids)
            }
            ast_internal::Expr::Match(m) => {
                let value_id = self.convert_expr(&m.value);
                let arm_ids: Vec<AstId> =
                    m.arms.iter().map(|a| self.convert_match_arm(a)).collect();
                let mut v = vec![value_id];
                v.extend(arm_ids);
                ("match", "match".into(), v)
            }
            ast_internal::Expr::Let(let_expr) => {
                let pat_id = self.convert_pattern(&let_expr.pattern);
                let value_id = self.convert_expr(&let_expr.value);
                ("let_expr", "let".into(), vec![pat_id, value_id])
            }
            ast_internal::Expr::Paren(p) => {
                let inner_id = self.convert_expr(p);
                ("paren", "( )".into(), vec![inner_id])
            }
            ast_internal::Expr::Err => ("expr_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, expr.span);
        id
    }

    fn convert_struct_expr_field(
        &mut self,
        field: &ast_internal::AstNode<ast_internal::StructExprField>,
    ) -> AstId {
        let id = field.ast_id.into();
        let name = field.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&field.node.ident);
        let expr_id = self.convert_expr(&field.node.expr);
        self.insert_node(
            id,
            "struct_expr_field",
            name,
            vec![ident_id, expr_id],
            field.span,
        );
        id
    }

    fn convert_match_arm(&mut self, arm: &ast_internal::AstNode<ast_internal::MatchArm>) -> AstId {
        let id = arm.ast_id.into();
        let pat_id = self.convert_pattern(&arm.node.pattern);
        let body_id = self.convert_expr(&arm.node.body);
        self.insert_node(
            id,
            "match_arm",
            "=>".into(),
            vec![pat_id, body_id],
            arm.span,
        );
        id
    }

    fn convert_block_expr(
        &mut self,
        block: &ast_internal::AstNode<ast_internal::BlockExpr>,
    ) -> AstId {
        self.convert_block(block)
    }

    fn convert_block(&mut self, block: &ast_internal::AstNode<ast_internal::BlockExpr>) -> AstId {
        let id = block.ast_id.into();
        let stmt_ids: Vec<AstId> = block
            .node
            .stmts
            .iter()
            .map(|s| self.convert_stmt(s))
            .collect();
        self.insert_node(id, "block", "{ }".into(), stmt_ids, block.span);
        id
    }

    fn convert_pattern(&mut self, pat: &ast_internal::AstNode<ast_internal::Pattern>) -> AstId {
        let id = pat.ast_id.into();
        let (kind, label, children) = match &pat.node {
            ast_internal::Pattern::Wildcard => ("pat_wildcard", "_".into(), vec![]),
            ast_internal::Pattern::Or(pats) => {
                let kids: Vec<AstId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                ("pat_or", "|".into(), kids)
            }
            ast_internal::Pattern::Path(p) => {
                let path_id = self.convert_path(p);
                ("pat_path", "path".into(), vec![path_id])
            }
            ast_internal::Pattern::Struct(path, fields) => {
                let path_id = self.convert_path(path);
                let fids: Vec<AstId> = fields
                    .iter()
                    .map(|f| self.convert_pat_struct_field(f))
                    .collect();
                let mut v = vec![path_id];
                v.extend(fids);
                ("pat_struct", "struct".into(), v)
            }
            ast_internal::Pattern::TupleStruct(path, pats) => {
                let path_id = self.convert_path(path);
                let kids: Vec<AstId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                let mut v = vec![path_id];
                v.extend(kids);
                ("pat_tuple_struct", "tuple_struct".into(), v)
            }
            ast_internal::Pattern::Tuple(pats) => {
                let kids: Vec<AstId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                ("pat_tuple", "()".into(), kids)
            }
            ast_internal::Pattern::Expr(e) => {
                let child = self.convert_expr(e);
                ("pat_expr", "expr".into(), vec![child])
            }
            ast_internal::Pattern::Paren(p) => {
                let child = self.convert_pattern(p);
                ("pat_paren", "( )".into(), vec![child])
            }
            ast_internal::Pattern::Err => ("pat_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, pat.span);
        id
    }

    fn convert_pat_struct_field(
        &mut self,
        field: &ast_internal::AstNode<ast_internal::PatternStructField>,
    ) -> AstId {
        let id = field.ast_id.into();
        let name = field.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&field.node.ident);
        let pat_id = self.convert_pattern(&field.node.pattern);
        self.insert_node(
            id,
            "pat_struct_field",
            name,
            vec![ident_id, pat_id],
            field.span,
        );
        id
    }

    fn convert_ty(&mut self, ty: &ast_internal::AstNode<ast_internal::Ty>) -> AstId {
        let id = ty.ast_id.into();
        let (kind, label, children) = match &ty.node {
            ast_internal::Ty::Path(p) => {
                let path_id = self.convert_path(p);
                ("ty_path", "path".into(), vec![path_id])
            }
            ast_internal::Ty::Array(elem_ty, len) => {
                let elem_id = self.convert_ty(elem_ty);
                let len_id = self.convert_expr(len);
                ("ty_array", "[]".into(), vec![elem_id, len_id])
            }
            ast_internal::Ty::Ptr(elem_ty) => {
                let elem_id = self.convert_ty(elem_ty);
                ("ty_ptr", "*".into(), vec![elem_id])
            }
            ast_internal::Ty::Fn(param_tys, return_ty) => {
                let param_ids: Vec<AstId> = param_tys.iter().map(|t| self.convert_ty(t)).collect();
                let return_id = return_ty.as_ref().as_ref().map(|t| self.convert_ty(t));
                let mut v = param_ids;
                if let Some(r) = return_id {
                    v.push(r);
                }
                ("ty_fn", "fn".into(), v)
            }
            ast_internal::Ty::Tuple(tys) => {
                let kids: Vec<AstId> = tys.iter().map(|t| self.convert_ty(t)).collect();
                ("ty_tuple", "()".into(), kids)
            }
            ast_internal::Ty::Paren(inner) => {
                let inner_id = self.convert_ty(inner);
                ("ty_paren", "( )".into(), vec![inner_id])
            }
            ast_internal::Ty::Err => ("ty_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, ty.span);
        id
    }

    fn convert_path(&mut self, path: &ast_internal::AstNode<ast_internal::Path>) -> AstId {
        let id = path.ast_id.into();
        let seg_ids: Vec<AstId> = path
            .node
            .segments
            .iter()
            .map(|s| self.convert_segment(s))
            .collect();
        let label = path
            .node
            .segments
            .iter()
            .map(|s| s.node.ident.node.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        self.insert_node(id, "path", label, seg_ids, path.span);
        id
    }

    fn convert_segment(
        &mut self,
        segment: &ast_internal::AstNode<ast_internal::PathSegment>,
    ) -> AstId {
        let id = segment.ast_id.into();
        let name = segment.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&segment.node.ident);
        let arg_ids: Vec<AstId> = segment
            .node
            .args
            .iter()
            .map(|a| self.convert_generic_arg(a))
            .collect();
        let mut children = vec![ident_id];
        children.extend(arg_ids);
        self.insert_node(id, "path_segment", name, children, segment.span);
        id
    }

    fn convert_generic_arg(
        &mut self,
        arg: &ast_internal::AstNode<ast_internal::GenericArg>,
    ) -> AstId {
        let id = arg.ast_id.into();
        let (kind, children) = match &arg.node {
            ast_internal::GenericArg::Type(ty) => ("generic_arg_type", vec![self.convert_ty(ty)]),
            ast_internal::GenericArg::Const(expr) => {
                ("generic_arg_const", vec![self.convert_expr(expr)])
            }
        };
        self.insert_node(id, kind, "".into(), children, arg.span);
        id
    }

    fn convert_ident(&mut self, ident: &ast_internal::AstNode<ast_internal::Ident>) -> AstId {
        let id = ident.ast_id.into();
        let name = ident.node.name.clone();
        self.insert_node(id, "ident", name, vec![], ident.span);
        id
    }
}
