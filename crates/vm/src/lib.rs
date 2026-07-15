//! Bytecode VM for rscpp.

mod chunk;
mod compile;
mod error;
mod vm;

pub use chunk::{Chunk, Op, Program};
pub use compile::compile;
pub use error::VmError;
pub use vm::{call, run_main, Vm};
