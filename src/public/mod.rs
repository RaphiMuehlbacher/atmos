mod ast;
mod defs;
mod hir;
mod span;
mod token;

pub use ast::{AstCrate, AstId, AstNode};
pub use defs::{DefId, DefKind, Definition, PartialRes, PrimTy, Res, SelfTyInfo};
pub use hir::{HirCrate, HirId, HirNode};
pub use span::Span;
pub use token::{
    DelimiterKind, KeywordKind, PublicLiteral, PublicToken, PublicTokenKind, PunctuationKind,
};

use std::collections::HashMap;

use crate::session::session::Output;
use serde::{Deserialize, Serialize};

fn convert_res(internal: &crate::resolver::ribs::Res) -> Res {
    use crate::resolver::ribs::Res as I;
    match internal {
        I::Local(ast_id) => Res::Local(AstId(ast_id.0)),
        I::Def(def_id, kind) => Res::Def(DefId(def_id.0), convert_def_kind(kind)),
        I::PrimTy(p) => Res::PrimTy(match p {
            crate::resolver::ribs::PrimTy::I32 => PrimTy::I32,
            crate::resolver::ribs::PrimTy::U32 => PrimTy::U32,
            crate::resolver::ribs::PrimTy::F64 => PrimTy::F64,
            crate::resolver::ribs::PrimTy::Bool => PrimTy::Bool,
            crate::resolver::ribs::PrimTy::Str => PrimTy::Str,
        }),
        I::SelfTy(info) => Res::SelfTy(SelfTyInfo {
            self_ty_def: info.self_ty_def.map(|d| DefId(d.0)),
            trait_def: info.trait_def.map(|d| DefId(d.0)),
            impl_or_trait_def: DefId(info.impl_or_trait_def.0),
        }),
        I::Err => Res::Err,
    }
}

fn convert_def_kind(kind: &crate::resolver::defs::DefKind) -> DefKind {
    use crate::resolver::defs::DefKind as I;
    match kind {
        I::Struct => DefKind::Struct,
        I::StructField => DefKind::StructField,
        I::Enum => DefKind::Enum,
        I::EnumVariant => DefKind::EnumVariant,
        I::Trait => DefKind::Trait,
        I::Mod => DefKind::Mod,
        I::Impl => DefKind::Impl,
        I::Function => DefKind::Function,
        I::AssocFn => DefKind::AssocFn,
        I::ExternFn => DefKind::ExternFn,
        I::Use => DefKind::Use,
        I::Const => DefKind::Const,
        I::GenericParam => DefKind::TypeParam,
        I::TypeAlias => DefKind::TypeAlias,
        I::AssocTypeAlias => DefKind::AssocTypeAlias,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicOutput {
    pub tokens: Vec<PublicToken>,
    pub ast: Option<AstCrate>,
    pub hir: Option<HirCrate>,
    pub ast_to_def: HashMap<AstId, DefId>,
    pub resolutions: HashMap<AstId, Res>,
    pub def_map: HashMap<DefId, DefKind>,
}

impl From<Output> for PublicOutput {
    fn from(output: Output) -> Self {
        let public_tokens: Vec<PublicToken> = output
            .tokens
            .unwrap_or_default()
            .iter()
            .map(|t| PublicToken {
                kind: PublicTokenKind::from(&t.kind),
                span: t.span.into(),
            })
            .collect();

        let public_ast = output.ast.as_ref().map(|krate| AstCrate::from_krate(krate));
        let public_hir = output.hir.as_ref().map(|krate| HirCrate::from_krate(krate));

        PublicOutput {
            tokens: public_tokens,
            ast: public_ast,
            hir: public_hir,
            ast_to_def: output
                .ast_to_def
                .iter()
                .map(|(k, v)| (AstId(k.0), DefId(v.0)))
                .collect(),
            resolutions: output
                .resolutions
                .iter()
                .map(|(k, v)| (AstId(k.0), convert_res(v)))
                .collect(),
            def_map: output
                .def_map
                .iter()
                .map(|(k, v)| (DefId(k.0), convert_def_kind(v)))
                .collect(),
        }
    }
}
