use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ast_lowerer::hir as hir_internal;

use super::span::Span;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HirId(pub usize);

impl From<hir_internal::HirId> for HirId {
    fn from(hir_id: hir_internal::HirId) -> Self {
        Self(hir_id.0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirCrate {
    pub root_items: Vec<HirId>,
    pub nodes: HashMap<HirId, HirNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirNode {
    pub id: HirId,
    pub span: Span,
    pub kind: String,
    pub label: String,
    pub children: Vec<HirId>,
}

impl HirCrate {
    pub fn from_krate(krate: &hir_internal::Crate) -> Self {
        let mut converter = HirConverter {
            nodes: HashMap::new(),
        };

        let root_items: Vec<HirId> = krate
            .items
            .iter()
            .map(|item| converter.convert_item(item))
            .collect();

        HirCrate {
            root_items,
            nodes: converter.nodes,
        }
    }
}

struct HirConverter {
    nodes: HashMap<HirId, HirNode>,
}

impl HirConverter {
    fn insert_node(
        &mut self,
        id: HirId,
        kind: &str,
        label: String,
        children: Vec<HirId>,
        span: impl Into<Span>,
    ) {
        self.nodes.insert(
            id,
            HirNode {
                id,
                span: span.into(),
                kind: kind.to_string(),
                label,
                children,
            },
        );
    }

    fn convert_item(&mut self, item: &hir_internal::HirNode<hir_internal::Item>) -> HirId {
        let id = item.hir_id.into();
        let (kind, label, _, children) = match &item.node {
            hir_internal::Item::Fn(f) => {
                let sig_id = self.convert_fn_sig(&f.sig);
                let body_id = self.convert_block_expr(&f.body);
                let name = f.sig.node.ident.node.name.clone();
                (
                    "fn",
                    format!("fn {name}"),
                    Some(f.def_id),
                    vec![sig_id, body_id],
                )
            }
            hir_internal::Item::Struct(s) => {
                let data_id = self.convert_variant_data(&s.data);
                let generics_ids: Vec<HirId> = s
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = s.ident.node.name.clone();
                let mut v = generics_ids;
                v.push(data_id);
                ("struct", format!("struct {name}"), Some(s.def_id), v)
            }
            hir_internal::Item::Enum(e) => {
                let variant_ids: Vec<HirId> = e
                    .variants
                    .iter()
                    .map(|v| self.convert_enum_variant(v))
                    .collect();
                let generics_ids: Vec<HirId> = e
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = e.ident.node.name.clone();
                let mut v = generics_ids;
                v.extend(variant_ids);
                ("enum", format!("enum {name}"), Some(e.def_id), v)
            }
            hir_internal::Item::Trait(t) => {
                let items_ids: Vec<HirId> = t
                    .items
                    .iter()
                    .map(|i| self.convert_associated_item(i))
                    .collect();
                let generics_ids: Vec<HirId> = t
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = t.ident.node.name.clone();
                let mut v = generics_ids;
                v.extend(items_ids);
                ("trait", format!("trait {name}"), Some(t.def_id), v)
            }
            hir_internal::Item::Mod(m) => {
                let items_ids: Vec<HirId> = m.items.iter().map(|i| self.convert_item(i)).collect();
                let name = m.ident.node.name.clone();
                ("mod", format!("mod {name}"), Some(m.def_id), items_ids)
            }
            hir_internal::Item::Impl(i) => {
                let self_ty_id = self.convert_ty(&i.self_ty);
                let trait_id = i.of_trait.as_ref().map(|t| self.convert_path(t));
                let items_ids: Vec<HirId> = i
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
                let label = if i.of_trait.is_some() {
                    "impl ... for ...".to_string()
                } else {
                    "impl".to_string()
                };
                ("impl", label, Some(i.def_id), children)
            }
            hir_internal::Item::ExternFn(f) => {
                let sig_id = self.convert_fn_sig(&f.sig);
                let name = f.sig.node.ident.node.name.clone();
                (
                    "extern_fn",
                    format!("extern fn {name}"),
                    Some(f.def_id),
                    vec![sig_id],
                )
            }
            hir_internal::Item::Const(c) => {
                let ty_id = self.convert_ty(&c.ty);
                let expr_id = self.convert_expr(&c.expr);
                let name = c.ident.node.name.clone();
                (
                    "const",
                    format!("const {name}"),
                    Some(c.def_id),
                    vec![ty_id, expr_id],
                )
            }
            hir_internal::Item::TyAlias(t) => {
                let ty_id = self.convert_ty(&t.ty);
                let generics_ids: Vec<HirId> = t
                    .generics
                    .iter()
                    .map(|g| self.convert_generic_param(g))
                    .collect();
                let name = t.ident.node.name.clone();
                let mut v = generics_ids;
                v.push(ty_id);
                ("type_alias", format!("type {name}"), Some(t.def_id), v)
            }
        };
        self.insert_node(id, kind, label, children, item.span);
        id
    }

    fn convert_fn_sig(&mut self, sig: &hir_internal::HirNode<hir_internal::FnSig>) -> HirId {
        let id = sig.hir_id.into();
        let name = sig.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&sig.node.ident);
        let generic_ids: Vec<HirId> = sig
            .node
            .generics
            .iter()
            .map(|g| self.convert_generic_param(g))
            .collect();
        let param_ids: Vec<HirId> = sig
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

    fn convert_param(&mut self, param: &hir_internal::HirNode<hir_internal::Param>) -> HirId {
        let id = param.hir_id.into();
        let pat_id = self.convert_pattern(&param.node.pattern);
        let ty_id = self.convert_ty(&param.node.type_annotation);
        self.insert_node(id, "param", "param".into(), vec![pat_id, ty_id], param.span);
        id
    }

    fn convert_generic_param(
        &mut self,
        gp: &hir_internal::HirNode<hir_internal::GenericParam>,
    ) -> HirId {
        let id = gp.hir_id.into();
        let name = gp.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&gp.node.ident);
        let bound_ids: Vec<HirId> = gp
            .node
            .bounds
            .iter()
            .map(|b| self.convert_path(b))
            .collect();
        let kind_child = match &gp.node.kind {
            hir_internal::GenericParamKind::Const(ty) => vec![self.convert_ty(ty)],
            hir_internal::GenericParamKind::Type => vec![],
        };
        let mut children = vec![ident_id];
        children.extend(bound_ids);
        children.extend(kind_child);
        self.insert_node(id, "generic_param", name, children, gp.span);
        id
    }

    fn convert_variant_data(
        &mut self,
        data: &hir_internal::HirNode<hir_internal::VariantData>,
    ) -> HirId {
        let id = data.hir_id.into();
        let (kind, label, children) = match &data.node {
            hir_internal::VariantData::Unit => ("variant_data_unit", "unit".into(), vec![]),
            hir_internal::VariantData::Struct { fields } => {
                let fids: Vec<HirId> = fields.iter().map(|f| self.convert_field_def(f)).collect();
                ("variant_data_struct", "struct".into(), fids)
            }
            hir_internal::VariantData::Tuple { fields } => {
                let fids: Vec<HirId> = fields.iter().map(|f| self.convert_field_def(f)).collect();
                ("variant_data_tuple", "tuple".into(), fids)
            }
        };
        self.insert_node(id, kind, label, children, data.span);
        id
    }

    fn convert_field_def(
        &mut self,
        field: &hir_internal::HirNode<hir_internal::FieldDef>,
    ) -> HirId {
        let id = field.hir_id.into();
        let name = field.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&field.node.ident);
        let ty_id = self.convert_ty(&field.node.ty);
        self.insert_node(id, "field_def", name, vec![ident_id, ty_id], field.span);
        id
    }

    fn convert_enum_variant(
        &mut self,
        variant: &hir_internal::HirNode<hir_internal::EnumVariant>,
    ) -> HirId {
        let id = variant.hir_id.into();
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
        item: &hir_internal::HirNode<hir_internal::AssociatedItem>,
    ) -> HirId {
        let id = item.hir_id.into();
        let (kind, label, _def_id, children) = match &item.node.kind {
            hir_internal::AssociatedItemKind::Fn(sig, body) => {
                let sig_id = self.convert_fn_sig(sig);
                let body_id = body.as_ref().map(|b| self.convert_block_expr(b));
                let mut v = vec![sig_id];
                if let Some(b) = body_id {
                    v.push(b);
                }
                ("assoc_fn", "fn".into(), Some(item.node.parent), v)
            }
            hir_internal::AssociatedItemKind::Type(alias) => {
                let ident_id = self.convert_ident(&alias.node.ident);
                let ty_id = alias.node.ty.as_ref().map(|t| self.convert_ty(t));
                let generics_ids: Vec<HirId> = alias
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
                ("assoc_type", "type".into(), Some(alias.node.def_id), v)
            }
        };
        self.insert_node(id, kind, label, children, item.span);
        id
    }

    fn convert_stmt(&mut self, stmt: &hir_internal::HirNode<hir_internal::Stmt>) -> HirId {
        let id = stmt.hir_id.into();
        let (kind, label, children) = match &stmt.node {
            hir_internal::Stmt::Let(let_stmt) => {
                let pat_id = self.convert_pattern(&let_stmt.pattern);
                let ty_id = let_stmt.ty.as_ref().map(|t| self.convert_ty(t));
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
            hir_internal::Stmt::Expr(expr) => {
                let child = self.convert_expr(expr);
                ("stmt_expr", "expr".into(), vec![child])
            }
            hir_internal::Stmt::Semi(expr) => {
                let child = self.convert_expr(expr);
                ("stmt_semi", "semi".into(), vec![child])
            }
            hir_internal::Stmt::Item(item) => {
                let child = self.convert_item(item);
                ("stmt_item", "item".into(), vec![child])
            }
        };
        self.insert_node(id, kind, label, children, stmt.span);
        id
    }

    fn convert_expr(&mut self, expr: &hir_internal::HirNode<hir_internal::Expr>) -> HirId {
        let id = expr.hir_id.into();
        let (kind, label, children) = match &expr.node {
            hir_internal::Expr::Array(elems) => {
                let kids: Vec<HirId> = elems.iter().map(|e| self.convert_expr(e)).collect();
                ("array", "[]".into(), kids)
            }
            hir_internal::Expr::Struct(s) => {
                let path_id = self.convert_path(&s.path);
                let field_ids: Vec<HirId> = s
                    .fields
                    .iter()
                    .map(|f| self.convert_struct_expr_field(f))
                    .collect();
                let mut v = vec![path_id];
                v.extend(field_ids);
                ("struct_expr", "struct".into(), v)
            }
            hir_internal::Expr::Call(c) => {
                let callee_id = self.convert_expr(&c.callee);
                let arg_ids: Vec<HirId> = c.args.iter().map(|a| self.convert_expr(a)).collect();
                let mut v = vec![callee_id];
                v.extend(arg_ids);
                ("call", "call".into(), v)
            }
            hir_internal::Expr::MethodCall(m) => {
                let receiver_id = self.convert_expr(&m.receiver);
                let method_id = self.convert_segment(&m.method);
                let arg_ids: Vec<HirId> = m.args.iter().map(|a| self.convert_expr(a)).collect();
                let mut v = vec![receiver_id, method_id];
                v.extend(arg_ids);
                ("method_call", "method_call".into(), v)
            }
            hir_internal::Expr::Tuple(elems) => {
                let kids: Vec<HirId> = elems.iter().map(|e| self.convert_expr(e)).collect();
                ("tuple", "()".into(), kids)
            }
            hir_internal::Expr::Cast(c) => {
                let expr_id = self.convert_expr(&c.expr);
                let ty_id = self.convert_ty(&c.ty);
                ("cast", "as".into(), vec![expr_id, ty_id])
            }
            hir_internal::Expr::Return(v) => {
                let child = v.as_ref().map(|e| self.convert_expr(e));
                let mut v = Vec::new();
                if let Some(c) = child {
                    v.push(c);
                }
                ("return", "return".into(), v)
            }
            hir_internal::Expr::Loop(l) => {
                let body_id = self.convert_block_expr(&l.body);
                ("loop", "loop".into(), vec![body_id])
            }
            hir_internal::Expr::Assign(a) => {
                let lhs_id = self.convert_expr(&a.lhs);
                let rhs_id = self.convert_expr(&a.rhs);
                ("assign", "=".into(), vec![lhs_id, rhs_id])
            }
            hir_internal::Expr::Field(f) => {
                let base_id = self.convert_expr(&f.base);
                let field_id = self.convert_ident(&f.field);
                ("field", ".".into(), vec![base_id, field_id])
            }
            hir_internal::Expr::Index(i) => {
                let base_id = self.convert_expr(&i.base);
                let index_id = self.convert_expr(&i.index);
                ("index", "[]".into(), vec![base_id, index_id])
            }
            hir_internal::Expr::Path(p) => {
                let path_id = self.convert_path(p);
                ("path", "path".into(), vec![path_id])
            }
            hir_internal::Expr::AddrOf(inner) => {
                let inner_id = self.convert_expr(inner);
                ("addr_of", "&".into(), vec![inner_id])
            }
            hir_internal::Expr::Break(v) => {
                let child = v.as_ref().map(|e| self.convert_expr(e));
                let mut v = Vec::new();
                if let Some(c) = child {
                    v.push(c);
                }
                ("break", "break".into(), v)
            }
            hir_internal::Expr::Continue => ("continue", "continue".into(), vec![]),
            hir_internal::Expr::Literal(l) => {
                let label = match l {
                    hir_internal::Literal::Bool(b) => b.to_string(),
                    hir_internal::Literal::I32(i) => i.to_string(),
                    hir_internal::Literal::U32(u) => u.to_string(),
                    hir_internal::Literal::F64(f) => f.to_string(),
                    hir_internal::Literal::Str(s) => format!("\"{s}\""),
                    hir_internal::Literal::Unit => "()".into(),
                };
                ("literal", label, vec![])
            }
            hir_internal::Expr::Binary(b) => {
                let lhs_id = self.convert_expr(&b.lhs);
                let rhs_id = self.convert_expr(&b.rhs);
                let op_label = format!("{:?}", b.op);
                ("binary", op_label, vec![lhs_id, rhs_id])
            }
            hir_internal::Expr::Unary(u) => {
                let operand_id = self.convert_expr(&u.operand);
                let op_label = format!("{:?}", u.op);
                ("unary", op_label, vec![operand_id])
            }
            hir_internal::Expr::If(if_expr) => {
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
            hir_internal::Expr::Block(b) => {
                let body_id = self.convert_block_expr(b);
                ("block", "block".into(), vec![body_id])
            }
            hir_internal::Expr::Match(m) => {
                let scrutinee_id = self.convert_expr(&m.scrutinee);
                let arm_ids: Vec<HirId> =
                    m.arms.iter().map(|a| self.convert_match_arm(a)).collect();
                let mut v = vec![scrutinee_id];
                v.extend(arm_ids);
                ("match", "match".into(), v)
            }
            hir_internal::Expr::Let(let_expr) => {
                let pat_id = self.convert_pattern(&let_expr.pattern);
                let init_id = self.convert_expr(&let_expr.init);
                ("let_expr", "let".into(), vec![pat_id, init_id])
            }
            hir_internal::Expr::Err => ("expr_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, expr.span);
        id
    }

    fn convert_struct_expr_field(
        &mut self,
        field: &hir_internal::HirNode<hir_internal::StructExprField>,
    ) -> HirId {
        let id = field.hir_id.into();
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

    fn convert_match_arm(&mut self, arm: &hir_internal::HirNode<hir_internal::MatchArm>) -> HirId {
        let id = arm.hir_id.into();
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
        block: &hir_internal::HirNode<hir_internal::BlockExpr>,
    ) -> HirId {
        let id = block.hir_id.into();
        let stmt_ids: Vec<HirId> = block
            .node
            .stmts
            .iter()
            .map(|s| self.convert_stmt(s))
            .collect();
        self.insert_node(id, "block", "{ }".into(), stmt_ids, block.span);
        id
    }

    fn convert_pattern(&mut self, pat: &hir_internal::HirNode<hir_internal::Pattern>) -> HirId {
        let id = pat.hir_id.into();
        let (kind, label, children) = match &pat.node {
            hir_internal::Pattern::Wildcard => ("pat_wildcard", "_".into(), vec![]),
            hir_internal::Pattern::Or(pats) => {
                let kids: Vec<HirId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                ("pat_or", "|".into(), kids)
            }
            hir_internal::Pattern::Path(p) => {
                let path_id = self.convert_path(p);
                ("pat_path", "path".into(), vec![path_id])
            }
            hir_internal::Pattern::Struct(path, fields) => {
                let path_id = self.convert_path(path);
                let fids: Vec<HirId> = fields
                    .iter()
                    .map(|f| self.convert_pat_struct_field(f))
                    .collect();
                let mut v = vec![path_id];
                v.extend(fids);
                ("pat_struct", "struct".into(), v)
            }
            hir_internal::Pattern::TupleStruct(path, pats) => {
                let path_id = self.convert_path(path);
                let kids: Vec<HirId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                let mut v = vec![path_id];
                v.extend(kids);
                ("pat_tuple_struct", "tuple_struct".into(), v)
            }
            hir_internal::Pattern::Tuple(pats) => {
                let kids: Vec<HirId> = pats.iter().map(|p| self.convert_pattern(p)).collect();
                ("pat_tuple", "()".into(), kids)
            }
            hir_internal::Pattern::Expr(e) => {
                let child = self.convert_expr(e);
                ("pat_expr", "expr".into(), vec![child])
            }
            hir_internal::Pattern::Err => ("pat_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, pat.span);
        id
    }

    fn convert_pat_struct_field(
        &mut self,
        field: &hir_internal::HirNode<hir_internal::PatternStructField>,
    ) -> HirId {
        let id = field.hir_id.into();
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

    fn convert_ty(&mut self, ty: &hir_internal::HirNode<hir_internal::Ty>) -> HirId {
        let id = ty.hir_id.into();
        let (kind, label, children) = match &ty.node {
            hir_internal::Ty::Path(p) => {
                let path_id = self.convert_path(p);
                ("ty_path", "path".into(), vec![path_id])
            }
            hir_internal::Ty::Array(elem_ty, len) => {
                let elem_id = self.convert_ty(elem_ty);
                let len_id = self.convert_expr(len);
                ("ty_array", "[]".into(), vec![elem_id, len_id])
            }
            hir_internal::Ty::Ptr(elem_ty) => {
                let elem_id = self.convert_ty(elem_ty);
                ("ty_ptr", "*".into(), vec![elem_id])
            }
            hir_internal::Ty::Fn(param_tys, return_ty) => {
                let param_ids: Vec<HirId> = param_tys.iter().map(|t| self.convert_ty(t)).collect();
                let return_id = return_ty.as_ref().map(|t| self.convert_ty(t));
                let mut v = param_ids;
                if let Some(r) = return_id {
                    v.push(r);
                }
                ("ty_fn", "fn".into(), v)
            }
            hir_internal::Ty::Tuple(tys) => {
                let kids: Vec<HirId> = tys.iter().map(|t| self.convert_ty(t)).collect();
                ("ty_tuple", "()".into(), kids)
            }
            hir_internal::Ty::Err => ("ty_err", "err".into(), vec![]),
        };
        self.insert_node(id, kind, label, children, ty.span);
        id
    }

    fn convert_path(&mut self, path: &hir_internal::HirNode<hir_internal::Path>) -> HirId {
        let id = path.hir_id.into();
        let segments = match &path.node {
            hir_internal::Path::Resolved { segments, .. } => segments,
            hir_internal::Path::Unresolved {
                resolved_segments,
                unresolved_segments,
                ..
            } => {
                let mut all = resolved_segments.clone();
                all.extend(unresolved_segments.clone());
                // We don't own the data, so iterate over references
                let seg_ids: Vec<HirId> = resolved_segments
                    .iter()
                    .chain(unresolved_segments.iter())
                    .map(|s| self.convert_segment(s))
                    .collect();
                let label = resolved_segments
                    .iter()
                    .chain(unresolved_segments.iter())
                    .map(|s| s.node.ident.node.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                self.insert_node(id, "path", label, seg_ids, path.span);
                return id;
            }
        };
        let seg_ids: Vec<HirId> = segments.iter().map(|s| self.convert_segment(s)).collect();
        let label = segments
            .iter()
            .map(|s| s.node.ident.node.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        self.insert_node(id, "path", label, seg_ids, path.span);
        id
    }

    fn convert_segment(
        &mut self,
        segment: &hir_internal::HirNode<hir_internal::PathSegment>,
    ) -> HirId {
        let id = segment.hir_id.into();
        let name = segment.node.ident.node.name.clone();
        let ident_id = self.convert_ident(&segment.node.ident);
        let arg_ids: Vec<HirId> = segment
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
        arg: &hir_internal::HirNode<hir_internal::GenericArg>,
    ) -> HirId {
        let id = arg.hir_id.into();
        let children = match &arg.node {
            hir_internal::GenericArg::Type(ty) => vec![self.convert_ty(ty)],
            hir_internal::GenericArg::Const(expr) => vec![self.convert_expr(expr)],
        };
        self.insert_node(id, "generic_arg", "".into(), children, arg.span);
        id
    }

    fn convert_ident(&mut self, ident: &hir_internal::HirNode<crate::parser::ast::Ident>) -> HirId {
        let id = ident.hir_id.into();
        let name = ident.node.name.clone();
        self.insert_node(id, "ident", name, vec![], ident.span);
        id
    }
}
