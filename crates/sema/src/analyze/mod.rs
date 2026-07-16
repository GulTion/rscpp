//! Semantic analyzer: scopes, type resolution, light checking.

use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

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
}

impl Context {
    fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            errors: Vec::new(),
            current_class: None,
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
            "queue",
            "stack",
            "priority_queue",
            "string",
            "iostream",
            "size_t",
            "int64_t",
            "uint64_t",
            "int32_t",
            "uint32_t",
            "numeric_limits",
        ] {
            let _ = self.symbols.define(Symbol {
                name: name.into(),
                ty: Ty::named(name, vec![]),
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
            ("make_shared", Ty::Unknown),
            ("make_unique", Ty::Unknown),
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
        for name in ["min", "max"] {
            let _ = self.symbols.redefine(Symbol {
                name: name.into(),
                ty: Ty::Function {
                    ret: Box::new(Ty::Int),
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

mod declare;
mod check;
mod expr;
mod types;
mod stl;
