use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

pub(crate) fn stl_member(name: &str, args: &[Ty], field: &str) -> Option<Ty> {
    match (name, field) {
        ("vector", "size") | ("string", "size") | ("string", "length")
        | ("map", "size") | ("unordered_map", "size")
        | ("set", "size") | ("queue", "size") | ("stack", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("vector", "empty") | ("string", "empty") | ("map", "empty") | ("queue", "empty")
        | ("stack", "empty") => Some(Ty::Function {
            ret: Box::new(Ty::Bool),
            params: vec![],
        }),
        ("vector", "push_back") | ("vector", "emplace_back") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![elem],
            })
        }
        ("vector", "pop_back") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "clear") | ("map", "clear") | ("string", "clear") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "begin") | ("vector", "end") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![],
        }),
        ("map", "insert") | ("unordered_map", "insert") | ("set", "insert")
        | ("unordered_set", "insert") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![Ty::Unknown],
        }),
        ("map", "count")
        | ("unordered_map", "count")
        | ("set", "count")
        | ("unordered_set", "count") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Int),
                params: vec![key],
            })
        }
        ("map", "erase")
        | ("unordered_map", "erase")
        | ("set", "erase")
        | ("unordered_set", "erase") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![key],
            })
        }
        ("map", "find") | ("unordered_map", "find") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Unknown),
                params: vec![key],
            })
        }
        ("queue", "push") | ("stack", "push") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![elem],
            })
        }
        ("queue", "front") | ("queue", "back") | ("stack", "top") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![],
            })
        }
        ("queue", "pop") | ("stack", "pop") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("priority_queue", "push") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![elem],
            })
        }
        ("priority_queue", "top") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![],
            })
        }
        ("priority_queue", "pop") | ("priority_queue", "empty") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("priority_queue", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("pair", "first") => Some(args.first().cloned().unwrap_or(Ty::Unknown)),
        ("pair", "second") => Some(args.get(1).cloned().unwrap_or(Ty::Unknown)),
        ("string", "c_str") => Some(Ty::Function {
            ret: Box::new(Ty::Pointer(Box::new(Ty::Char))),
            params: vec![],
        }),
        ("ostream", "operator<<") | ("istream", "operator>>") => Some(Ty::Unknown),
        _ => None,
    }
}
