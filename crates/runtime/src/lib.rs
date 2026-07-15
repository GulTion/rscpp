//! Tree-walking C++ interpreter with event stream.

mod engine;
mod error;
mod event;
mod value;

pub use engine::Engine;
pub use error::RuntimeError;
pub use event::Event;
pub use value::{Heap, Object, ObjId, Value};
