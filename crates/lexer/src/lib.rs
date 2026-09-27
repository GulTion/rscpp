//! Hand-written C++ lexer for the rscpp interpreter (LeetCode subset).

mod diag;
mod error;
mod scanner;
mod span;
mod token;

pub use diag::{format_diagnostic, line_col};
pub use error::LexError;
pub use scanner::tokenize;
pub use span::Span;
pub use token::{FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind};
