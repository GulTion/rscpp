use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

pub(crate) fn stl_member(name: &str, args: &[Ty], field: &str) -> Option<Ty> {
    match (name, field) {
        ("vector", "size") | ("string", "size") | ("string", "length")
        | ("map", "size") | ("unordered_map", "size")
        | ("set", "size") | ("unordered_set", "size")
        | ("queue", "size") | ("stack", "size")
        | ("deque", "size") | ("list", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("vector", "empty") | ("string", "empty") | ("map", "empty") | ("queue", "empty")
        | ("stack", "empty") | ("deque", "empty") | ("unordered_set", "empty")
        | ("set", "empty") => Some(Ty::Function {
            ret: Box::new(Ty::Bool),
            params: vec![],
        }),
        ("vector", "push_back") | ("vector", "emplace_back")
        | ("deque", "push_back") | ("deque", "emplace_back")
        | ("deque", "push_front") | ("string", "push_back") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "pop_back") | ("deque", "pop_back") | ("deque", "pop_front")
        | ("string", "pop_back") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "front") | ("vector", "back")
        | ("deque", "front") | ("deque", "back")
        | ("string", "front") | ("string", "back")
        | ("list", "front") | ("list", "back") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![],
            })
        },
        ("vector", "clear") | ("map", "clear") | ("string", "clear")
        | ("deque", "clear") | ("set", "clear") | ("unordered_set", "clear") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "reserve") | ("string", "reserve") | ("unordered_map", "reserve")
        | ("unordered_set", "reserve") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![Ty::UInt],
        }),
        ("vector", "resize") | ("string", "resize") | ("deque", "resize") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![], // 1 or 2 args
        }),
        ("vector", "begin") | ("vector", "end")
        | ("vector", "cbegin") | ("vector", "cend")
        | ("string", "begin") | ("string", "end")
        | ("string", "cbegin") | ("string", "cend")
        | ("string", "rbegin") | ("string", "rend")
        | ("map", "begin") | ("map", "end")
        | ("unordered_map", "begin") | ("unordered_map", "end")
        | ("set", "begin") | ("set", "end")
        | ("set", "cbegin") | ("set", "cend")
        | ("unordered_set", "begin") | ("unordered_set", "end")
        | ("unordered_set", "cbegin") | ("unordered_set", "cend")
        | ("deque", "begin") | ("deque", "end") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![],
        }),
        ("string", "find") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![Ty::Unknown],
        }),
        ("string", "substr") => Some(Ty::Function {
            ret: Box::new(Ty::named("string", vec![])),
            params: vec![], // variadic 1–3
        }),
        ("string", "append") | ("string", "replace") => Some(Ty::Function {
            ret: Box::new(Ty::named("string", vec![])),
            params: vec![],
        }),
        ("string", "npos") => Some(Ty::UInt),
        ("string", "at") | ("vector", "at") | ("unordered_map", "at") | ("map", "at")
        | ("deque", "at") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![Ty::Unknown],
            })
        },
        ("map", "insert") | ("unordered_map", "insert") | ("set", "insert")
        | ("unordered_set", "insert") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![Ty::Unknown],
        }),
        ("map", "emplace") | ("unordered_map", "emplace") | ("set", "emplace")
        | ("unordered_set", "emplace") | ("stack", "emplace") | ("queue", "emplace")
        | ("priority_queue", "emplace") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![], // variadic
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
        ("numeric_limits", "max") | ("numeric_limits", "min") | ("numeric_limits", "lowest") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Int),
                params: vec![],
            })
        }
        ("ostream", "operator<<") | ("istream", "operator>>") => Some(Ty::Unknown),
        _ => None,
    }
}
