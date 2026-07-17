//! Tree-walking C++ interpreter with event stream.

mod builtins;
mod engine;
mod error;
mod event;
mod json_args;
pub mod stl;
mod value;

pub use engine::{Engine, DEFAULT_FUEL};
pub use error::RuntimeError;
pub use event::{AllocEntry, Event, Slot};
pub use json_args::{args_from_json, value_from_json};
pub use value::{Address, Heap, MapKey, ObjId, Object, Value};
