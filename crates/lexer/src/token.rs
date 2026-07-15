use crate::span::Span;

/// A single lexeme with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Decoded token kinds for the LeetCode C++ subset.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    IntLit {
        value: u128,
        suffix: IntSuffix,
        base: IntBase,
    },
    FloatLit {
        value: f64,
        suffix: FloatSuffix,
    },
    CharLit(char),
    StringLit(String),
    Punct(Punct),
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntBase {
    Decimal,
    Hex,
    Binary,
    Octal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IntSuffix {
    #[default]
    None,
    U,
    L,
    Ul,
    Ll,
    Ull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FloatSuffix {
    #[default]
    None,
    F,
    L,
}

/// Keywords recognized by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Alignas,
    Alignof,
    Auto,
    Bool,
    Break,
    Case,
    Catch,
    Char,
    Class,
    Const,
    Constexpr,
    Continue,
    Decltype,
    Default,
    Delete,
    Do,
    Double,
    Else,
    Enum,
    Explicit,
    Extern,
    False,
    Float,
    For,
    Friend,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Mutable,
    Namespace,
    New,
    Noexcept,
    Nullptr,
    Operator,
    Private,
    Protected,
    Public,
    Register,
    ReinterpretCast,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    StaticAssert,
    StaticCast,
    Struct,
    Switch,
    Template,
    This,
    Throw,
    True,
    Try,
    Typedef,
    Typeid,
    Typename,
    Union,
    Unsigned,
    Using,
    Virtual,
    Void,
    Volatile,
    WcharT,
    While,
    Override,
    Final,
}

impl Keyword {
    /// Map an identifier spelling to a keyword, if it is one.
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "alignas" => Self::Alignas,
            "alignof" => Self::Alignof,
            "auto" => Self::Auto,
            "bool" => Self::Bool,
            "break" => Self::Break,
            "case" => Self::Case,
            "catch" => Self::Catch,
            "char" => Self::Char,
            "class" => Self::Class,
            "const" => Self::Const,
            "constexpr" => Self::Constexpr,
            "continue" => Self::Continue,
            "decltype" => Self::Decltype,
            "default" => Self::Default,
            "delete" => Self::Delete,
            "do" => Self::Do,
            "double" => Self::Double,
            "else" => Self::Else,
            "enum" => Self::Enum,
            "explicit" => Self::Explicit,
            "extern" => Self::Extern,
            "false" => Self::False,
            "float" => Self::Float,
            "for" => Self::For,
            "friend" => Self::Friend,
            "goto" => Self::Goto,
            "if" => Self::If,
            "inline" => Self::Inline,
            "int" => Self::Int,
            "long" => Self::Long,
            "mutable" => Self::Mutable,
            "namespace" => Self::Namespace,
            "new" => Self::New,
            "noexcept" => Self::Noexcept,
            "nullptr" => Self::Nullptr,
            "operator" => Self::Operator,
            "private" => Self::Private,
            "protected" => Self::Protected,
            "public" => Self::Public,
            "register" => Self::Register,
            "reinterpret_cast" => Self::ReinterpretCast,
            "return" => Self::Return,
            "short" => Self::Short,
            "signed" => Self::Signed,
            "sizeof" => Self::Sizeof,
            "static" => Self::Static,
            "static_assert" => Self::StaticAssert,
            "static_cast" => Self::StaticCast,
            "struct" => Self::Struct,
            "switch" => Self::Switch,
            "template" => Self::Template,
            "this" => Self::This,
            "throw" => Self::Throw,
            "true" => Self::True,
            "try" => Self::Try,
            "typedef" => Self::Typedef,
            "typeid" => Self::Typeid,
            "typename" => Self::Typename,
            "union" => Self::Union,
            "unsigned" => Self::Unsigned,
            "using" => Self::Using,
            "virtual" => Self::Virtual,
            "void" => Self::Void,
            "volatile" => Self::Volatile,
            "wchar_t" => Self::WcharT,
            "while" => Self::While,
            "override" => Self::Override,
            "final" => Self::Final,
            _ => return None,
        })
    }
}

/// Punctuation and operators (maximal-munch spellings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Punct {
    // Single
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Semi,
    Colon,
    Comma,
    Dot,
    Question,
    Tilde,
    Not,
    Hash,
    // Multi / ops
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Amp,
    Pipe,
    Eq,
    Lt,
    Gt,
    PlusPlus,
    MinusMinus,
    Arrow,
    ArrowStar,
    DotStar,
    Scope,
    LtLt,
    GtGt,
    LtEq,
    GtEq,
    EqEq,
    NotEq,
    AmpAmp,
    PipePipe,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    CaretEq,
    AmpEq,
    PipeEq,
    LtLtEq,
    GtGtEq,
    Ellipsis,
}
