use crate::extension::SourceSpanExt;
use crate::parser::ast::{AstNode, Ident};
use crate::resolver::ribs::Res;
use crate::resolver::DefId;
use miette::SourceSpan;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirId(usize);

static mut HIR_ID: usize = 0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirNode<T> {
    pub node: T,
    pub span: SourceSpan,
    pub hir_id: HirId,
}

impl<T> HirNode<T> {
    pub fn new(node: T, span: SourceSpan) -> Self {
        let hir_id = Self::fresh_hir_id();
        Self::with_id(node, span, hir_id)
    }

    pub fn with_id(node: T, span: SourceSpan, hir_id: HirId) -> Self {
        Self { node, span, hir_id }
    }

    pub fn err(node: T) -> Self {
        HirNode::new(node, SourceSpan::err_span())
    }

    pub fn fresh_hir_id() -> HirId {
        unsafe {
            let id = HIR_ID;
            HIR_ID += 1;
            HirId(id)
        }
    }
}

impl From<AstNode<Ident>> for HirNode<Ident> {
    fn from(value: AstNode<Ident>) -> Self {
        HirNode::new(value.node, value.span)
    }
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub items: Vec<HirNode<Item>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<HirNode<PathSegment>>,
    pub res: Res,
}

#[derive(Debug, Clone)]
pub struct PathSegment {
    pub ident: HirNode<Ident>,
    pub args: Vec<HirNode<GenericArg>>,
}

#[derive(Debug, Clone)]
pub enum GenericArg {
    /// `Bar` in `Foo<Bar>`
    Type(HirNode<Ty>),
    /// `1` in `Foo<const 1>`
    Const(Box<HirNode<Expr>>),
}

#[derive(Debug, Clone)]
pub enum Ty {
    Path(HirNode<Path>),
    Array(Box<HirNode<Ty>>, Box<HirNode<Expr>>),
    Ptr(Box<HirNode<Ty>>),
    Fn(Vec<HirNode<Ty>>, Option<Box<HirNode<Ty>>>),
    Tuple(Vec<HirNode<Ty>>),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Or(Vec<HirNode<Pattern>>),
    Path(HirNode<Path>),
    Struct(HirNode<Path>, Vec<HirNode<PatternStructField>>),
    TupleStruct(HirNode<Path>, Vec<HirNode<Pattern>>),
    Tuple(Vec<HirNode<Pattern>>),
    Expr(Box<HirNode<Expr>>),
}

#[derive(Debug, Clone)]
pub struct PatternStructField {
    pub ident: HirNode<Ident>,
    pub pattern: HirNode<Pattern>,
}

#[derive(Debug, Clone)]
pub enum AssociatedItem {
    Fn(HirNode<FnSig>, Option<HirNode<BlockExpr>>),
    Type(HirNode<TyAlias>),
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Mod(ModDecl),
    Impl(ImplDecl),
    ExternFn(ExternFnDecl),
    Const(ConstDecl),
    TyAlias(TyAlias),
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub def_id: DefId,
    pub sig: HirNode<FnSig>,
    pub body: HirNode<BlockExpr>,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub params: Vec<HirNode<Param>>,
    pub return_ty: Option<HirNode<Ty>>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: HirNode<Pattern>,
    pub type_annotation: HirNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub bounds: Vec<HirNode<Path>>,
    pub kind: GenericParamKind,
}

#[derive(Debug, Clone)]
pub enum GenericParamKind {
    Const(HirNode<Ty>),
    Type,
}

#[derive(Debug, Clone)]
pub struct ExternFnDecl {
    pub def_id: DefId,
    pub sig: HirNode<FnSig>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub ty: Option<HirNode<Ty>>,
    pub expr: HirNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub data: HirNode<VariantData>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub variants: Vec<HirNode<EnumVariant>>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub data: HirNode<VariantData>,
}

#[derive(Debug, Clone)]
pub enum VariantData {
    Unit,
    Struct { fields: Vec<HirNode<StructField>> },
    Tuple { types: Vec<HirNode<Ty>> },
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub ty: HirNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub items: Vec<HirNode<AssociatedItem>>,
}

#[derive(Debug, Clone)]
pub struct ModDecl {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub items: Vec<HirNode<Item>>,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub def_id: DefId,
    pub generics: Vec<HirNode<GenericParam>>,
    pub self_ty: HirNode<Ty>,
    pub of_trait: Option<HirNode<Path>>,
    pub items: Vec<HirNode<AssociatedItem>>,
}

#[derive(Debug, Clone)]
pub struct TyAlias {
    pub def_id: DefId,
    pub ident: HirNode<Ident>,
    pub generics: Vec<HirNode<GenericParam>>,
    pub ty: Option<HirNode<Ty>>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(LetStmt),
    Expr(HirNode<Expr>),
    Semi(HirNode<Expr>),
    Item(HirNode<Item>),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub pattern: HirNode<Pattern>,
    pub ty: Option<HirNode<Ty>>,
    pub expr: Option<Box<HirNode<Expr>>>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Array(Vec<HirNode<Expr>>),
    Struct(StructExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Tuple(Vec<HirNode<Expr>>),
    Cast(CastExpr),
    Return(Option<Box<HirNode<Expr>>>),
    Loop(LoopExpr),
    Assign(AssignExpr),
    Field(FieldExpr),
    Index(IndexExpr),
    Path(HirNode<Path>),
    AddrOf(Box<HirNode<Expr>>),
    Break(Option<Box<HirNode<Expr>>>),
    Continue,
    Literal(Literal),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    If(IfExpr),
    Block(HirNode<BlockExpr>),
    Match(MatchExpr),
    Let(LetExpr),
    Err,
}

#[derive(Debug, Clone)]
pub struct StructExpr {
    pub path: HirNode<Path>,
    pub fields: Vec<HirNode<StructExprField>>,
}

#[derive(Debug, Clone)]
pub struct StructExprField {
    pub ident: HirNode<Ident>,
    pub expr: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<HirNode<Expr>>,
    pub args: Vec<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct MethodCallExpr {
    pub receiver: Box<HirNode<Expr>>,
    pub method: HirNode<PathSegment>,
    pub args: Vec<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct CastExpr {
    pub expr: Box<HirNode<Expr>>,
    pub ty: HirNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct LoopExpr {
    pub body: HirNode<BlockExpr>,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub lhs: Box<HirNode<Expr>>,
    pub rhs: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct FieldExpr {
    pub base: Box<HirNode<Expr>>,
    pub field: HirNode<Ident>,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub base: Box<HirNode<Expr>>,
    pub index: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Bool(bool),
    I32(i32),
    U32(u32),
    F64(f64),
    Str(String),
    Unit,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub lhs: Box<HirNode<Expr>>,
    pub op: BinOp,
    pub rhs: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnOp,
    pub operand: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Deref,
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<HirNode<Expr>>,
    pub then_branch: HirNode<BlockExpr>,
    pub else_branch: Option<HirNode<BlockExpr>>,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub stmts: Vec<HirNode<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub scrutinee: Box<HirNode<Expr>>,
    pub arms: Vec<HirNode<MatchArm>>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: HirNode<Pattern>,
    pub body: Box<HirNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct LetExpr {
    pub pattern: HirNode<Pattern>,
    pub init: Box<HirNode<Expr>>,
}
