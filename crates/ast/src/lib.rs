//! Typed AST for the LeetCode C++ subset.

pub use rscpp_lexer::Span;

/// Top-level parse result.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslationUnit {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Class(ClassDef),
    Function(FunctionDef),
    UsingNamespace {
        path: Path,
        span: Span,
    },
    Decl(Decl),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Struct,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub kind: ClassKind,
    pub name: Ident,
    pub members: Vec<Member>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Access(AccessSpec),
    Function(FunctionDef),
    Field(Decl),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSpec {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub return_type: Type,
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: Type,
    pub name: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub ty: Type,
    pub declarators: Vec<InitDeclarator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitDeclarator {
    pub name: Ident,
    /// Extra pointer/ref layers on this declarator (beyond those on `ty`).
    pub ptrs: Vec<PtrKind>,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtrKind {
    Pointer,
    Reference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Block),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
        span: Span,
    },
    For {
        init: Option<ForInit>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Box<Stmt>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
    Decl(Decl),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    Decl(Decl),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLit {
        value: u128,
        span: Span,
    },
    FloatLit {
        value: f64,
        span: Span,
    },
    CharLit {
        value: char,
        span: Span,
    },
    StringLit {
        value: String,
        span: Span,
    },
    BoolLit {
        value: bool,
        span: Span,
    },
    Nullptr {
        span: Span,
    },
    Name(Path),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Assign {
        op: AssignOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Member {
        base: Box<Expr>,
        field: Ident,
        arrow: bool,
        span: Span,
    },
    Cast {
        ty: Type,
        expr: Box<Expr>,
        span: Span,
    },
    InitList {
        elems: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    Deref,
    AddrOf,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Builtin {
        kind: BuiltinType,
        span: Span,
    },
    Named {
        path: Path,
        args: Vec<Type>,
        span: Span,
    },
    Pointer {
        inner: Box<Type>,
        span: Span,
    },
    Reference {
        inner: Box<Type>,
        span: Span,
    },
    Const {
        inner: Box<Type>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinType {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    LongLong,
    UnsignedChar,
    UnsignedShort,
    UnsignedInt,
    UnsignedLong,
    UnsignedLongLong,
    Float,
    Double,
    WcharT,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl Path {
    pub fn single(ident: Ident) -> Self {
        let span = ident.span;
        Self {
            segments: vec![ident],
            span,
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::IntLit { span, .. }
            | Self::FloatLit { span, .. }
            | Self::CharLit { span, .. }
            | Self::StringLit { span, .. }
            | Self::BoolLit { span, .. }
            | Self::Nullptr { span }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Assign { span, .. }
            | Self::Call { span, .. }
            | Self::Index { span, .. }
            | Self::Member { span, .. }
            | Self::Cast { span, .. }
            | Self::InitList { span, .. } => *span,
            Self::Name(p) => p.span,
        }
    }
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Self::Builtin { span, .. }
            | Self::Named { span, .. }
            | Self::Pointer { span, .. }
            | Self::Reference { span, .. }
            | Self::Const { span, .. } => *span,
        }
    }
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Self::Block(b) => b.span,
            Self::If { span, .. }
            | Self::While { span, .. }
            | Self::DoWhile { span, .. }
            | Self::For { span, .. }
            | Self::Return { span, .. }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Expr { span, .. } => *span,
            Self::Decl(d) => d.span,
        }
    }
}
