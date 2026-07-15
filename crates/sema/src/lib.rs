//! Semantic analysis for the rscpp interpreter (LeetCode subset).

mod analyze;
mod error;
mod symbols;
mod ty;

pub use analyze::{analyze, SemaResult};
pub use error::SemaError;
pub use ty::Ty;
