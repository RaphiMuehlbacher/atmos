use crate::extension::SourceSpanExt;
use miette::SourceSpan;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AstId(usize);

static mut AST_ID: usize = 0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AstNode<T> {
    pub node: T,
    pub span: SourceSpan,
    pub ast_id: AstId,
}

impl<T> AstNode<T> {
    pub fn new(node: T, span: SourceSpan) -> Self {
        let ast_id = Self::fresh_ast_id();
        Self { node, span, ast_id }
    }

    pub fn err(node: T) -> Self {
        AstNode::new(node, SourceSpan::err_span())
    }

    pub fn fresh_ast_id() -> AstId {
        unsafe {
            let id = AST_ID;
            AST_ID += 1;
            AstId(id)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    name: String,
}

impl Ident {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    pub fn err() -> Self {
        Self {
            name: "err".to_string(),
        }
    }
}

impl PartialEq<&str> for Ident {
    fn eq(&self, other: &&str) -> bool {
        self.name == *other
    }
}

/// e.g. std::cmp::PartialEq
#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<AstNode<PathSegment>>,
}

/// e.g. std, String, Box<T>
#[derive(Debug, Clone)]
pub struct PathSegment {
    pub ident: AstNode<Ident>,
    pub args: Vec<AstNode<GenericArg>>,
}

#[derive(Debug, Clone)]
pub enum GenericArg {
    /// `Bar` in `Foo<Bar>`
    Type(AstNode<Ty>),
    /// `1` in `Foo<const 1>`
    Const(Box<AstNode<Expr>>),
}

#[derive(Debug, Clone)]
pub enum Ty {
    Path(AstNode<Path>),
    Array(Box<AstNode<Ty>>, Box<AstNode<Expr>>),
    Ptr(Box<AstNode<Ty>>),
    Fn(Vec<AstNode<Ty>>, Box<Option<AstNode<Ty>>>),
    Tuple(Vec<AstNode<Ty>>),
    Paren(Box<AstNode<Ty>>),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Or(Vec<AstNode<Pattern>>),
    Ident(AstNode<Ident>),
    Struct(AstNode<Path>, Vec<AstNode<PatternStructField>>),
    TupleStruct(AstNode<Path>, Vec<AstNode<Pattern>>),
    Tuple(Vec<AstNode<Pattern>>),
    Expr(Box<AstNode<Expr>>),
    Paren(Box<AstNode<Pattern>>),
}

#[derive(Debug, Clone)]
pub struct PatternStructField {
    pub ident: AstNode<Ident>,
    pub pattern: AstNode<Pattern>,
}

#[derive(Debug, Clone)]
pub enum AssociatedItem {
    Fn(Box<AstNode<FnDecl>>),
    Type(Box<AstNode<TyAliasDecl>>),
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub items: Vec<AstNode<Item>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Item(AstNode<Item>),
    Let(LetStmt),
    Expr(AstNode<Expr>),
    Semi(AstNode<Expr>),
    Err,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    ExternFn(ExternFnDecl),
    Const(ConstDecl),
    Use(UseItem),
    TyAlias(TyAliasDecl),
    Err,
}

#[derive(Debug, Clone)]
pub struct UseItem {
    pub path: AstNode<Path>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub sig: AstNode<FnSig>,
    pub body: AstNode<BlockExpr>,
}

#[derive(Debug, Clone)]
pub struct ExternFnDecl {
    /// Span of the 'extern' keyword
    pub span: SourceSpan,
    pub sig: AstNode<FnSig>,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub ident: AstNode<Ident>,
    pub generics: Vec<AstNode<GenericParam>>,
    pub params: Vec<AstNode<Param>>,
    pub return_ty: Option<AstNode<Ty>>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: AstNode<Pattern>,
    pub type_annotation: AstNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub ident: AstNode<Ident>,
    pub generics: Vec<GenericParam>,
    pub type_annotation: Option<AstNode<Ty>>,
    pub expr: Option<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub ident: AstNode<Ident>,
    pub bounds: Vec<AstNode<Path>>,
    pub kind: GenericParamKind,
}

#[derive(Debug, Clone)]
pub enum GenericParamKind {
    Const(AstNode<Ty>),
    Type,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub ident: AstNode<Ident>,
    pub generics: Vec<AstNode<GenericParam>>,
    pub variants: Vec<AstNode<EnumVariant>>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub ident: AstNode<Ident>,
    pub data: AstNode<VariantData>,
}

#[derive(Debug, Clone)]
pub enum VariantData {
    Unit,
    Struct {
        fields: Vec<AstNode<StructFieldDef>>,
    },
    Tuple {
        types: Vec<AstNode<Ty>>,
    },
}

#[derive(Debug, Clone)]
pub struct StructFieldDef {
    pub ident: AstNode<Ident>,
    pub type_annotation: AstNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub ident: AstNode<Ident>,
    pub data: AstNode<VariantData>,
    pub generics: Vec<AstNode<GenericParam>>,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub ident: AstNode<Ident>,
    pub generics: Vec<AstNode<GenericParam>>,
    pub items: Vec<AstNode<AssociatedItem>>,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub ident: AstNode<Ident>,
    pub generics: Vec<AstNode<GenericParam>>,
    pub for_trait: Option<AstNode<Path>>,
    pub items: Vec<AstNode<AssociatedItem>>,
}

#[derive(Debug, Clone)]
pub struct TyAliasDecl {
    pub ident: AstNode<Ident>,
    pub generics: Vec<AstNode<GenericParam>>,
    pub ty: AstNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub pat: AstNode<Pattern>,
    pub type_annotation: Option<AstNode<Ty>>,
    pub expr: Option<Box<AstNode<Expr>>>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Array(ArrayExpr),
    Struct(StructExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Tuple(TupleExpr),
    Cast(CastExpr),
    Return(ReturnExpr),
    While(WhileExpr),
    Loop(LoopExpr),
    For(ForExpr),
    Assign(AssignExpr),
    AssignOp(AssignOpExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    Path(PathExpr),
    AddrOf(AddrOfExpr),
    Break(BreakExpr),
    Continue,
    Literal(LiteralExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    If(IfExpr),
    Block(BlockExpr),
    Match(MatchExpr),
    Let(LetExpr),
    Err,
    Paren(Box<AstNode<Expr>>),
}

#[derive(Debug, Clone)]
pub struct ArrayExpr {
    pub expressions: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct StructExpr {
    pub name: AstNode<Path>,
    pub fields: Vec<AstNode<StructExprField>>,
}

#[derive(Debug, Clone)]
pub struct StructExprField {
    pub ident: AstNode<Ident>,
    pub expr: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<AstNode<Expr>>,
    pub arguments: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct MethodCallExpr {
    pub name: AstNode<PathSegment>,
    pub receiver: Box<AstNode<Expr>>,
    pub arguments: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct TupleExpr {
    pub expressions: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct CastExpr {
    pub expr: Box<AstNode<Expr>>,
    pub ty: AstNode<Ty>,
}

#[derive(Debug, Clone)]
pub struct ReturnExpr {
    pub value: Option<Box<AstNode<Expr>>>,
}

#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub condition: Box<AstNode<Expr>>,
    pub body: Box<AstNode<BlockExpr>>,
}

#[derive(Debug, Clone)]
pub struct LoopExpr {
    pub body: Box<AstNode<BlockExpr>>,
}

#[derive(Debug, Clone)]
pub struct ForExpr {
    pub pattern: AstNode<Pattern>,
    pub iterator: Box<AstNode<Expr>>,
    pub body: Box<AstNode<BlockExpr>>,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub target: Box<AstNode<Expr>>,
    pub value: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct AssignOpExpr {
    pub op: AssignOp,
    pub target: Box<AstNode<Expr>>,
    pub value: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct FieldAccessExpr {
    pub target: Box<AstNode<Expr>>,
    pub field: AstNode<Ident>,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub target: Box<AstNode<Expr>>,
    pub index: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct PathExpr {
    pub path: AstNode<Path>,
}

#[derive(Debug, Clone)]
pub struct AddrOfExpr {
    pub expr: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct BreakExpr {
    pub expr: Option<Box<AstNode<Expr>>>,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub stmts: Vec<AstNode<Stmt>>,
}

#[derive(Debug, Clone)]
pub enum LiteralExpr {
    Bool(bool),
    I32(i32),
    U32(u32),
    F64(f64),
    Str(String),
    Unit,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<AstNode<Expr>>,
    pub operator: AstNode<BinOp>,
    pub right: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: AstNode<UnOp>,
    pub operand: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<AstNode<Expr>>,
    pub then_branch: Box<AstNode<BlockExpr>>,
    pub else_branch: Option<Box<AstNode<BlockExpr>>>,
}

#[derive(Debug, Clone)]
pub struct MatchExpr {
    pub value: Box<AstNode<Expr>>,
    pub arms: Vec<AstNode<MatchArm>>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: AstNode<Pattern>,
    pub body: Box<AstNode<Expr>>,
}

/// only semantically valid in the if / while condition
#[derive(Debug, Clone)]
pub struct LetExpr {
    pub pattern: AstNode<Pattern>,
    pub value: Box<AstNode<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    EqEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    NotEq,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Deref,
    Not,
    Neg,
    AddrOf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
}
