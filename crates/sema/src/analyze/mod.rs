//! Semantic analyzer: scopes, type resolution, light checking.

use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;
use std::collections::HashMap;

pub struct SemaResult {
    pub errors: Vec<SemaError>,
}

impl SemaResult {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Analyze a parsed translation unit.
pub fn analyze(tu: &TranslationUnit) -> SemaResult {
    let mut cx = Context::new();
    cx.seed_stl();
    cx.analyze_tu(tu);
    SemaResult { errors: cx.errors }
}

struct Context {
    symbols: SymbolTable,
    errors: Vec<SemaError>,
    /// Currently enclosing class name, if any.
    current_class: Option<String>,
    /// `using Alias = Type;`
    type_aliases: HashMap<String, Type>,
}

impl Context {
    fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            errors: Vec::new(),
            current_class: None,
            type_aliases: HashMap::new(),
        }
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(SemaError::new(span, msg));
    }

    fn seed_stl(&mut self) {
        // Types exist as empty class symbols; members resolved via lookup_member.
        for name in [
            "vector",
            "pair",
            "map",
            "unordered_map",
            "set",
            "unordered_set",
            "multiset",
            "unordered_multiset",
            "queue",
            "stack",
            "priority_queue",
            "string",
            "iostream",
            "numeric_limits",
            // Comparators / containers / streams used unqualified in LeetCode
            "greater",
            "less",
            "greater_equal",
            "less_equal",
            "equal_to",
            "not_equal_to",
            "plus",
            "minus",
            "multiplies",
            "divides",
            "modulus",
            "negate",
            "logical_and",
            "logical_or",
            "logical_not",
            "bit_and",
            "bit_or",
            "bit_xor",
            "bit_not",
            "bitset",
            "tuple",
            "stringstream",
            "istringstream",
            "ostringstream",
            "unique_lock",
            "mutex",
            "lock_guard",
            "condition_variable",
            "numbers",
            "array",
            "function",
            "Interval",
            // LeetCode node types (defs often only in comments)
            "Node",
            "TreeNode",
            "ListNode",
            "PolyNode",
            "UndirectedGraphNode",
            "TreeLinkNode",
        ] {
            let _ = self.symbols.define(Symbol {
                name: name.into(),
                ty: Ty::named(name, vec![]),
                kind: SymbolKind::Class,
            });
        }
        // Fixed-width / size aliases must resolve to numeric types (not opaque Named).
        for (name, ty) in [
            ("int64_t", Ty::LongLong),
            ("uint64_t", Ty::ULongLong),
            ("size_t", Ty::ULongLong),
            ("int32_t", Ty::Int),
            ("uint32_t", Ty::UInt),
            ("int16_t", Ty::Int),
            ("uint16_t", Ty::UInt),
            ("int8_t", Ty::Char),
            ("uint8_t", Ty::Char),
        ] {
            let _ = self.symbols.define(Symbol {
                name: name.into(),
                ty,
                kind: SymbolKind::Class,
            });
        }
        // Common free names
        let _ = self.symbols.define(Symbol {
            name: "std".into(),
            ty: Ty::named("std", vec![]),
            kind: SymbolKind::Class,
        });
        let _ = self.symbols.define(Symbol {
            name: "cin".into(),
            ty: Ty::named("istream", vec![]),
            kind: SymbolKind::Var,
        });
        let _ = self.symbols.define(Symbol {
            name: "cout".into(),
            ty: Ty::named("ostream", vec![]),
            kind: SymbolKind::Var,
        });
        let _ = self.symbols.define(Symbol {
            name: "endl".into(),
            ty: Ty::named("endl_t", vec![]),
            kind: SymbolKind::Var,
        });
        let _ = self.symbols.define(Symbol {
            name: "swap".into(),
            ty: Ty::Function {
                ret: Box::new(Ty::Void),
                params: vec![Ty::Unknown, Ty::Unknown],
            },
            kind: SymbolKind::Func,
        });
        // Free functions with correct-ish return types (variadic = empty params).
        let string_ty = Ty::named("string", vec![]);
        for (name, ret) in [
            ("to_string", string_ty.clone()),
            ("move", Ty::Unknown),
            ("forward", Ty::Unknown),
            ("begin", Ty::Unknown),
            ("end", Ty::Unknown),
            ("cbegin", Ty::Unknown),
            ("cend", Ty::Unknown),
            ("rbegin", Ty::Unknown),
            ("rend", Ty::Unknown),
            ("sort", Ty::Void),
            ("stable_sort", Ty::Void),
            ("reverse", Ty::Void),
            ("iota", Ty::Void),
            ("fill", Ty::Void),
            ("accumulate", Ty::Int),
            ("max_element", Ty::Pointer(Box::new(Ty::Unknown))),
            ("min_element", Ty::Pointer(Box::new(Ty::Unknown))),
            ("lower_bound", Ty::Unknown),
            ("upper_bound", Ty::Unknown),
            ("binary_search", Ty::Bool),
            ("tie", Ty::Unknown),
            ("make_tuple", Ty::Unknown),
            ("make_pair", Ty::Unknown),
            ("distance", Ty::Int),
            ("gcd", Ty::Int),
            ("lcm", Ty::Int),
            ("all_of", Ty::Bool),
            ("any_of", Ty::Bool),
            ("none_of", Ty::Bool),
            ("nth_element", Ty::Void),
            ("partial_sort", Ty::Void),
            ("make_shared", Ty::Unknown),
            ("make_unique", Ty::Unknown),
            ("__builtin_clz", Ty::Int),
            ("__builtin_clzll", Ty::Int),
            ("__builtin_ctz", Ty::Int),
            // <algorithm> / iterators / cassert / cmath extras
            ("transform", Ty::Unknown),
            ("bind", Ty::Unknown),
            ("assert", Ty::Void),
            ("count_if", Ty::Int),
            ("find_if", Ty::Unknown),
            ("find", Ty::Unknown),
            ("count", Ty::Int),
            ("rotate", Ty::Unknown),
            ("unique", Ty::Unknown),
            ("prev", Ty::Unknown),
            ("next", Ty::Unknown),
            ("back_inserter", Ty::Unknown),
            ("inserter", Ty::Unknown),
            ("front_inserter", Ty::Unknown),
            ("cref", Ty::Unknown),
            ("ref", Ty::Unknown),
            ("srand", Ty::Void),
            ("rand", Ty::Int),
            ("acos", Ty::Double),
            ("asin", Ty::Double),
            ("atan", Ty::Double),
            ("atan2", Ty::Double),
            ("log10", Ty::Double),
            ("log2", Ty::Double),
            ("log", Ty::Double),
            ("bit_width", Ty::Int),
            ("popcount", Ty::Int),
            ("time", Ty::LongLong),
            ("random_shuffle", Ty::Void),
            ("shuffle", Ty::Void),
            ("partial_sum", Ty::Unknown),
            ("stoull", Ty::ULongLong),
            ("stoul", Ty::ULong),
            ("bind1st", Ty::Unknown),
            ("bind2nd", Ty::Unknown),
            ("copy", Ty::Unknown),
            ("copy_if", Ty::Unknown),
            ("copy_n", Ty::Unknown),
            ("get", Ty::Unknown),
            ("getline", Ty::Unknown),
            ("make_heap", Ty::Void),
            ("push_heap", Ty::Void),
            ("pop_heap", Ty::Void),
            ("sort_heap", Ty::Void),
            ("set_intersection", Ty::Unknown),
            ("set_union", Ty::Unknown),
            ("set_difference", Ty::Unknown),
            ("next_permutation", Ty::Bool),
            ("prev_permutation", Ty::Bool),
            ("is_sorted", Ty::Bool),
            ("crbegin", Ty::Unknown),
            ("crend", Ty::Unknown),
            ("__lg", Ty::Int),
            ("llabs", Ty::LongLong),
            ("uniform_int_distribution", Ty::Unknown),
            ("default_random_engine", Ty::Unknown),
            ("mt19937", Ty::Unknown),
            ("plus", Ty::Unknown),
            ("multiplies", Ty::Unknown),
            ("bit_xor", Ty::Unknown),
            ("not_equal_to", Ty::Unknown),
            ("replace", Ty::Void),
        ] {
            self.symbols.redefine(Symbol {
                name: name.into(),
                ty: Ty::Function {
                    ret: Box::new(ret),
                    params: vec![], // soft variadic
                },
                kind: SymbolKind::Func,
            });
        }
        for (name, arity) in [
            ("min", 2),
            ("max", 2),
            ("abs", 1),
            ("pow", 2),
            ("sqrt", 1),
            ("ceil", 1),
            ("floor", 1),
            ("__builtin_popcount", 1),
            ("__builtin_popcountll", 1),
            ("size", 1),
            ("empty", 1),
            ("isdigit", 1),
            ("isalpha", 1),
            ("isalnum", 1),
            ("isupper", 1),
            ("islower", 1),
            ("isspace", 1),
            ("toupper", 1),
            ("tolower", 1),
            ("stoi", 1),
            ("stol", 1),
            ("stoll", 1),
            ("stod", 1),
        ] {
            let params = vec![Ty::Unknown; arity];
            let _ = self.symbols.define(Symbol {
                name: name.into(),
                ty: Ty::Function {
                    ret: Box::new(Ty::Int),
                    params,
                },
                kind: SymbolKind::Func,
            });
        }
        // Soft: min/max also allow 1-arg or 3+ via empty overload — redefine as variadic.
        // Return type Unknown so `min(vector, vector)` stays a vector, not int.
        for name in ["min", "max"] {
            let _ = self.symbols.redefine(Symbol {
                name: name.into(),
                ty: Ty::Function {
                    ret: Box::new(Ty::Unknown),
                    params: vec![],
                },
                kind: SymbolKind::Func,
            });
        }
        // `size` / `empty` free functions are overloaded — soft variadic.
        for (name, ret) in [("size", Ty::UInt), ("empty", Ty::Bool)] {
            let _ = self.symbols.redefine(Symbol {
                name: name.into(),
                ty: Ty::Function {
                    ret: Box::new(ret),
                    params: vec![],
                },
                kind: SymbolKind::Func,
            });
        }
        for name in [
            "INT_MAX",
            "INT_MIN",
            "LONG_MAX",
            "LONG_MIN",
            "LLONG_MAX",
            "LLONG_MIN",
            "UINT_MAX",
        ] {
            let _ = self.symbols.define(Symbol {
                name: name.into(),
                ty: Ty::Int,
                kind: SymbolKind::Var,
            });
        }
    }
}

mod check;
mod declare;
mod expr;
mod stl;
mod types;
