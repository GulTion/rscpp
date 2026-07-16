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
        | ("multiset", "size") | ("unordered_multiset", "size")
        | ("queue", "size") | ("stack", "size")
        | ("deque", "size") | ("list", "size")
        | ("array", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("vector", "empty") | ("string", "empty")
        | ("map", "empty") | ("unordered_map", "empty")
        | ("queue", "empty") | ("stack", "empty") | ("deque", "empty")
        | ("unordered_set", "empty") | ("set", "empty")
        | ("multiset", "empty") | ("unordered_multiset", "empty")
        | ("array", "empty") => Some(Ty::Function {
            ret: Box::new(Ty::Bool),
            params: vec![],
        }),
        ("vector", "push_back") | ("vector", "emplace_back")
        | ("deque", "push_back") | ("deque", "emplace_back")
        | ("deque", "push_front") | ("deque", "emplace_front")
        | ("string", "push_back") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "pop_back") | ("deque", "pop_back") | ("deque", "pop_front")
        | ("string", "pop_back") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "erase") | ("string", "erase") | ("deque", "erase") | ("list", "erase") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Unknown),
                params: vec![], // iterator / range overloads
            })
        }
        ("vector", "assign") | ("string", "assign") | ("deque", "assign") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("vector", "insert") | ("string", "insert") | ("deque", "insert") | ("list", "insert") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Unknown),
                params: vec![],
            })
        }
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
        ("vector", "clear") | ("map", "clear") | ("unordered_map", "clear") | ("string", "clear")
        | ("deque", "clear") | ("set", "clear") | ("unordered_set", "clear")
        | ("multiset", "clear") => Some(Ty::Function {
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
        | ("vector", "crbegin") | ("vector", "crend")
        | ("vector", "rbegin") | ("vector", "rend")
        | ("string", "begin") | ("string", "end")
        | ("string", "cbegin") | ("string", "cend")
        | ("string", "rbegin") | ("string", "rend")
        | ("string", "crbegin") | ("string", "crend")
        | ("map", "begin") | ("map", "end")
        | ("map", "cbegin") | ("map", "cend")
        | ("unordered_map", "begin") | ("unordered_map", "end")
        | ("unordered_map", "cbegin") | ("unordered_map", "cend")
        | ("set", "begin") | ("set", "end")
        | ("set", "cbegin") | ("set", "cend")
        | ("multiset", "begin") | ("multiset", "end")
        | ("multiset", "cbegin") | ("multiset", "cend")
        | ("unordered_set", "begin") | ("unordered_set", "end")
        | ("unordered_set", "cbegin") | ("unordered_set", "cend")
        | ("deque", "begin") | ("deque", "end")
        | ("array", "begin") | ("array", "end")
        | ("array", "cbegin") | ("array", "cend") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![],
        }),
        ("string", "compare") => Some(Ty::Function {
            ret: Box::new(Ty::Int),
            params: vec![], // overloads
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
        ("string", "starts_with") | ("string", "ends_with") | ("string", "contains") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Bool),
                params: vec![], // 1+ overloads
            })
        }
        ("string", "find") | ("string", "rfind") | ("string", "find_first_not_of")
        | ("string", "find_first_of") | ("string", "find_last_of")
        | ("string", "find_last_not_of") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![], // 1–3 arg overloads
        }),
        ("string", "at") | ("vector", "at") | ("deque", "at") | ("array", "at") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![Ty::Unknown],
            })
        },
        ("map", "at") | ("unordered_map", "at") => {
            let elem = args.get(1).cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![Ty::Unknown],
            })
        },
        ("array", "front") | ("array", "back") | ("array", "data") | ("array", "fill") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(elem),
                params: vec![],
            })
        }
        ("map", "insert") | ("unordered_map", "insert") | ("set", "insert")
        | ("unordered_set", "insert") | ("multiset", "insert") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![], // overloads
        }),
        ("map", "emplace") | ("unordered_map", "emplace") | ("set", "emplace")
        | ("unordered_set", "emplace") | ("multiset", "emplace")
        | ("stack", "emplace") | ("queue", "emplace")
        | ("priority_queue", "emplace")
        | ("vector", "emplace") | ("deque", "emplace") | ("list", "emplace") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![], // variadic
        }),
        ("map", "count")
        | ("unordered_map", "count")
        | ("set", "count")
        | ("unordered_set", "count")
        | ("multiset", "count") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Int),
                params: vec![key],
            })
        }
        ("map", "erase")
        | ("unordered_map", "erase")
        | ("set", "erase")
        | ("unordered_set", "erase")
        | ("multiset", "erase") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![key],
            })
        }
        ("map", "find") | ("unordered_map", "find")
        | ("set", "find") | ("unordered_set", "find") | ("multiset", "find") => {
            let key = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Function {
                ret: Box::new(Ty::Unknown),
                params: vec![key],
            })
        }
        ("set", "lower_bound") | ("set", "upper_bound")
        | ("multiset", "lower_bound") | ("multiset", "upper_bound")
        | ("map", "lower_bound") | ("map", "upper_bound") => {
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
        ("numeric_limits", "max") | ("numeric_limits", "min") | ("numeric_limits", "lowest")
        | ("numeric_limits", "infinity") | ("numeric_limits", "epsilon") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Double),
                params: vec![],
            })
        }
        ("ostream", "operator<<") | ("istream", "operator>>")
        | ("stringstream", "operator<<") | ("stringstream", "operator>>")
        | ("istringstream", "operator>>") | ("ostringstream", "operator<<") => Some(Ty::Unknown),
        ("bitset", "to_string") => Some(Ty::Function {
            ret: Box::new(Ty::named("string", vec![])),
            params: vec![],
        }),
        ("bitset", "count") | ("bitset", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("bitset", "test") | ("bitset", "any") | ("bitset", "none") | ("bitset", "all") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Bool),
                params: vec![],
            })
        }
        ("bitset", "set") | ("bitset", "reset") | ("bitset", "flip") => Some(Ty::Function {
            ret: Box::new(Ty::named("bitset", vec![])),
            params: vec![],
        }),
        ("condition_variable", "wait")
        | ("condition_variable", "wait_for")
        | ("condition_variable", "wait_until") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("condition_variable", "notify_one") | ("condition_variable", "notify_all") => {
            Some(Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![],
            })
        }
        ("mutex", "lock") | ("mutex", "unlock") | ("mutex", "try_lock")
        | ("lock_guard", "lock") | ("unique_lock", "lock") | ("unique_lock", "unlock")
        | ("unique_lock", "owns_lock") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
            params: vec![],
        }),
        ("numbers", "pi") | ("numbers", "e") => Some(Ty::Double),
        _ => None,
    }
}
