//! C++ parser for the rscpp interpreter (LeetCode subset).

mod error;
mod parser;

pub use error::ParseError;
pub use parser::parse;
