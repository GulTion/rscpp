use super::stl::stl_member;
use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

impl Context {
    pub(super) fn lookup_path(&mut self, path: &Path) -> Ty {
        if path.segments.is_empty() {
            return Ty::Error;
        }
        if path.segments.len() == 1 {
            let id = &path.segments[0];
            if let Some(sym) = self.symbols.lookup(&id.name) {
                return sym.ty.clone();
            }
            // method unqualified: try current class
            if let Some(c) = &self.current_class.clone() {
                let q = format!("{c}::{}", id.name);
                if let Some(sym) = self.symbols.lookup(&q) {
                    return sym.ty.clone();
                }
            }
            self.err(
                id.span,
                format!("use of undeclared identifier `{}`", id.name),
            );
            return Ty::Error;
        }
        // std::vector / ranges::max — free names under a namespace alias
        let last = path.segments.last().unwrap();
        if path.segments.len() == 2 {
            let ns = path.segments[0].name.as_str();
            if ns == "std" || ns == "ranges" {
                if let Some(sym) = self.symbols.lookup(&last.name) {
                    return sym.ty.clone();
                }
                if ns == "ranges" {
                    return Ty::Function {
                        ret: Box::new(Ty::Unknown),
                        params: vec![],
                    };
                }
                return Ty::named(&last.name, vec![]);
            }
        }
        // std::ranges::max_element
        if path.segments.len() == 3
            && path.segments[0].name == "std"
            && path.segments[1].name == "ranges"
        {
            if let Some(sym) = self.symbols.lookup(&last.name) {
                return sym.ty.clone();
            }
            return Ty::Function {
                ret: Box::new(Ty::Unknown),
                params: vec![],
            };
        }
        // Class::static_member / nested name (e.g. numeric_limits::max)
        if path.segments.len() == 2 {
            let first = &path.segments[0];
            let base = if let Some(sym) = self.symbols.lookup(&first.name) {
                sym.ty.clone()
            } else {
                Ty::named(&first.name, vec![])
            };
            return self.lookup_member(&base, &last.name, last.span);
        }
        // Class::method
        let q = path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        if let Some(sym) = self.symbols.lookup(&q) {
            return sym.ty.clone();
        }
        self.err(path.span, format!("use of undeclared name `{q}`"));
        Ty::Error
    }

    pub(super) fn check_fn_call(&mut self, ct: &Ty, args: &[Expr], span: Span) -> Ty {
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a)).collect();
        self.check_fn_call_types(ct, &arg_tys, span)
    }

    pub(super) fn check_fn_call_types(&mut self, ct: &Ty, arg_tys: &[Ty], span: Span) -> Ty {
        match ct.strip_cv_ref() {
            Ty::Function { ret, params } => {
                // Empty params = soft variadic (emplace and imperfect STL stubs).
                if !params.is_empty() {
                    if params.len() != arg_tys.len() {
                        self.err(
                            span,
                            format!(
                                "call expects {} argument(s), got {}",
                                params.len(),
                                arg_tys.len()
                            ),
                        );
                    } else {
                        for (p, a) in params.iter().zip(arg_tys.iter()) {
                            if !self.assignable(p, a) && *a != Ty::Error {
                                self.err(
                                    span,
                                    format!("argument type `{a}` not compatible with `{p}`"),
                                );
                            }
                        }
                    }
                }
                *ret.clone()
            }
            Ty::Named { name, args: targs } => {
                // `std::function<Ret(...)>` — call yields Ret (or Unknown).
                if name == "function" {
                    return targs.first().cloned().unwrap_or(Ty::Unknown);
                }
                let _ = (name, targs, arg_tys);
                ct.strip_cv_ref().clone()
            }
            // Functional cast: `int64_t(x)`, `long long(x)`, etc.
            other if other.is_numeric() => other.clone(),
            Ty::Unknown | Ty::Error | Ty::Auto => Ty::Unknown,
            other => {
                self.err(span, format!("cannot call value of type `{other}`"));
                Ty::Error
            }
        }
    }

    pub(super) fn lookup_member(&mut self, base: &Ty, field: &str, span: Span) -> Ty {
        let base = base.strip_cv_ref();
        // Soft: allow `p->field` / `p.field` when `p` is still typed as pointer value.
        if let Ty::Pointer(inner) = base {
            return self.lookup_member(inner, field, span);
        }
        match base {
            Ty::Named { name, args } => {
                // Expand type aliases stored under Class symbols.
                if let Some(sym) = self.symbols.lookup(name) {
                    if matches!(sym.kind, SymbolKind::Class) {
                        let aliased = sym.ty.clone();
                        if !matches!(aliased.strip_cv_ref(), Ty::Named { name: n, .. } if n == name)
                        {
                            return self.lookup_member(&aliased, field, span);
                        }
                    }
                }
                // User class field/method
                let q = format!("{name}::{field}");
                if let Some(sym) = self.symbols.lookup(&q) {
                    return sym.ty.clone();
                }
                if let Some(ty) = stl_member(name, args, field) {
                    return ty;
                }
                // LeetCode problem types are often only in comments — seed real fields.
                match (name.as_str(), field) {
                    ("TreeNode", "val")
                    | ("ListNode", "val")
                    | ("Node", "val")
                    | ("PolyNode", "coefficient")
                    | ("PolyNode", "power")
                    | ("UndirectedGraphNode", "label") => return Ty::Int,
                    ("TreeNode", "left") | ("TreeNode", "right") => {
                        return Ty::Pointer(Box::new(Ty::named("TreeNode", vec![])));
                    }
                    ("ListNode", "next") | ("Node", "next") | ("PolyNode", "next") => {
                        return Ty::Pointer(Box::new(Ty::named(name, vec![])));
                    }
                    // Doubly-linked / random / parent variants of Node
                    ("Node", "prev")
                    | ("Node", "child")
                    | ("Node", "parent")
                    | ("Node", "random")
                    | ("Node", "left")
                    | ("Node", "right")
                    | ("Node", "topLeft")
                    | ("Node", "topRight")
                    | ("Node", "bottomLeft")
                    | ("Node", "bottomRight") => {
                        return Ty::Pointer(Box::new(Ty::named("Node", vec![])));
                    }
                    ("Node", "isLeaf") => return Ty::Bool,
                    ("Node", "children") => {
                        return Ty::named(
                            "vector",
                            vec![Ty::Pointer(Box::new(Ty::named("Node", vec![])))],
                        );
                    }
                    ("UndirectedGraphNode", "neighbors") => {
                        return Ty::named(
                            "vector",
                            vec![Ty::Pointer(Box::new(Ty::named(
                                "UndirectedGraphNode",
                                vec![],
                            )))],
                        );
                    }
                    ("Interval", "start") | ("Interval", "end") => return Ty::Int,
                    ("pair", "first") => {
                        return args.first().cloned().unwrap_or(Ty::Unknown);
                    }
                    ("pair", "second") => {
                        return args.get(1).cloned().unwrap_or(Ty::Unknown);
                    }
                    _ => {}
                }
                self.err(span, format!("no member `{field}` on type `{base}`"));
                Ty::Error
            }
            Ty::Unknown | Ty::Auto => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("member access on non-class type `{other}`"));
                Ty::Error
            }
        }
    }

    /// Element type for `a[i]` / `operator[]`. Sets are intentionally rejected.
    pub(super) fn elem_type(&mut self, base: &Ty, span: Span) -> Ty {
        match base.strip_cv_ref() {
            Ty::Named { name, args }
                if name == "vector" || name == "string" || name == "deque" || name == "array" =>
            {
                if name == "string" {
                    Ty::Char
                } else if let Some(t) = args.first() {
                    Ty::Reference(Box::new(t.clone()))
                } else {
                    Ty::Unknown
                }
            }
            Ty::Named { name, args } if name == "map" || name == "unordered_map" => {
                if let Some(v) = args.get(1) {
                    Ty::Reference(Box::new(v.clone()))
                } else {
                    Ty::Unknown
                }
            }
            Ty::Named { name, .. } if name == "bitset" => Ty::Bool,
            Ty::Named { name, .. }
                if name == "set"
                    || name == "unordered_set"
                    || name == "multiset"
                    || name == "unordered_multiset" =>
            {
                self.err(span, format!("type `{name}` does not provide operator[]"));
                Ty::Error
            }
            Ty::Pointer(inner) => *inner.clone(),
            Ty::Unknown | Ty::Auto => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("type `{other}` is not subscriptable"));
                Ty::Error
            }
        }
    }

    /// Element type for range-for `for (auto& x : c)`. Includes set/unordered_set.
    pub(super) fn range_elem_type(&mut self, base: &Ty, span: Span) -> Ty {
        match base.strip_cv_ref() {
            Ty::Named { name, args }
                if name == "vector" || name == "deque" || name == "list" || name == "array" =>
            {
                args.first().cloned().unwrap_or(Ty::Unknown)
            }
            Ty::Named { name, .. } if name == "string" => Ty::Char,
            Ty::Named { name, args }
                if name == "set"
                    || name == "unordered_set"
                    || name == "multiset"
                    || name == "unordered_multiset" =>
            {
                args.first().cloned().unwrap_or(Ty::Unknown)
            }
            Ty::Named { name, args } if name == "map" || name == "unordered_map" => {
                // range-for yields `pair<K,V>`
                Ty::named("pair", args.clone())
            }
            Ty::Named { .. } | Ty::Unknown | Ty::Auto => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("type `{other}` is not a range"));
                Ty::Error
            }
        }
    }

    pub(super) fn check_unary(&mut self, op: UnaryOp, t: &Ty, span: Span) -> Ty {
        match op {
            UnaryOp::Plus
            | UnaryOp::Minus
            | UnaryOp::PreInc
            | UnaryOp::PreDec
            | UnaryOp::PostInc
            | UnaryOp::PostDec => {
                if !t.is_numeric() && *t != Ty::Error && *t != Ty::Unknown {
                    self.err(span, format!("invalid unary op on `{t}`"));
                }
                t.clone()
            }
            UnaryOp::Not => Ty::Bool,
            UnaryOp::BitNot => {
                if !t.is_integral() && *t != Ty::Error {
                    self.err(span, format!("`~` requires integral type, got `{t}`"));
                }
                t.clone()
            }
            UnaryOp::Deref => match t.strip_cv_ref() {
                Ty::Pointer(inner) => *inner.clone(),
                Ty::Named { name, args }
                    if name == "shared_ptr" || name == "unique_ptr" || name == "optional" =>
                {
                    args.first().cloned().unwrap_or(Ty::Unknown)
                }
                // Structured bindings often leave `auto` until we improve deduction.
                Ty::Error | Ty::Unknown | Ty::Auto => Ty::Unknown,
                other => {
                    self.err(span, format!("cannot dereference `{other}`"));
                    Ty::Error
                }
            },
            UnaryOp::AddrOf => Ty::Pointer(Box::new(t.clone())),
        }
    }

    pub(super) fn check_binary(&mut self, op: BinaryOp, lt: &Ty, rt: &Ty, span: Span) -> Ty {
        use BinaryOp::*;
        // Soft: `auto` / deduced placeholders / template params participate in any binary op.
        if is_soft_operand(lt) || is_soft_operand(rt) {
            return match op {
                Lt | Gt | Le | Ge | Eq | Ne | And | Or => Ty::Bool,
                Comma => rt.clone(),
                _ => {
                    if is_soft_operand(lt) {
                        rt.strip_cv_ref().clone()
                    } else {
                        lt.strip_cv_ref().clone()
                    }
                }
            };
        }
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitXor | BitOr | Shl | Shr => {
                let ls = lt.strip_cv_ref();
                let rs = rt.strip_cv_ref();
                if matches!(ls, Ty::Named { name, .. } if name == "string")
                    || matches!(rs, Ty::Named { name, .. } if name == "string")
                {
                    return Ty::named("string", vec![]);
                }
                // bitset supports &, |, ^, <<, >>
                if matches!(op, BitAnd | BitXor | BitOr | Shl | Shr) {
                    if matches!(ls, Ty::Named { name, .. } if name == "bitset")
                        || matches!(rs, Ty::Named { name, .. } if name == "bitset")
                    {
                        return if matches!(ls, Ty::Named { name, .. } if name == "bitset") {
                            lt.strip_cv_ref().clone()
                        } else {
                            rt.strip_cv_ref().clone()
                        };
                    }
                }
                // iostream extract/insert: `ssa >> x`, `cout << x`
                if matches!(op, Shl | Shr) {
                    if let Ty::Named { name, .. } = ls {
                        if matches!(
                            name.as_str(),
                            "istream"
                                | "ostream"
                                | "iostream"
                                | "stringstream"
                                | "istringstream"
                                | "ostringstream"
                        ) {
                            return lt.strip_cv_ref().clone();
                        }
                    }
                }
                if (!lt.is_numeric() || !rt.is_numeric()) && *lt != Ty::Error && *rt != Ty::Error {
                    self.err(
                        span,
                        format!("invalid operands `{lt}` and `{rt}` to binary op"),
                    );
                }
                if matches!(ls, Ty::Float | Ty::Double) || matches!(rs, Ty::Float | Ty::Double) {
                    Ty::Double
                } else {
                    Ty::Int
                }
            }
            Lt | Gt | Le | Ge | Eq | Ne => Ty::Bool,
            And | Or => Ty::Bool,
            Comma => rt.clone(),
        }
    }

    pub(super) fn assignable(&self, dst: &Ty, src: &Ty) -> bool {
        if matches!(dst, Ty::Unknown | Ty::Error | Ty::Auto)
            || matches!(src, Ty::Unknown | Ty::Error)
        {
            return true;
        }
        // `auto*`, `const auto&`, nested auto — accept any side that still has auto
        if ty_contains_auto(dst) || ty_contains_auto(src) {
            return true;
        }
        let d = dst.strip_cv_ref();
        let s = src.strip_cv_ref();
        // `const auto& x = …` / `auto&` — stripped Auto accepts any initializer
        if matches!(d, Ty::Unknown | Ty::Error | Ty::Auto)
            || matches!(s, Ty::Unknown | Ty::Error | Ty::Auto)
        {
            return true;
        }
        if d == s {
            return true;
        }
        // `vector<int> v(n)` ctor call types as bare `vector` — allow same container name
        if let (Ty::Named { name: dn, .. }, Ty::Named { name: sn, .. }) = (d, s) {
            if dn == sn {
                return true;
            }
        }
        // numeric promotions / conversions (loose)
        if d.is_numeric() && s.is_numeric() {
            return true;
        }
        // `bitset<N> b = 0;` / `bitset<N> b(val)` — construct from integer
        if matches!(d, Ty::Named { name, .. } if name == "bitset") && s.is_numeric() {
            return true;
        }
        // pointer: T* <- nullptr (void*), or compatible pointee
        if let (Ty::Pointer(a), Ty::Pointer(b)) = (d, s) {
            if matches!(b.as_ref(), Ty::Void) {
                return true;
            }
            if self.assignable(a, b) {
                return true;
            }
        }
        // Template type-params like `T` are opaque Unknown-ish
        if let Ty::Named { name, args } = d {
            if args.is_empty()
                && matches!(
                    name.as_str(),
                    "T" | "U" | "V" | "K" | "E" | "R" | "Cmp" | "Pred" | "Alloc"
                )
            {
                return true;
            }
        }
        if let Ty::Named { name, args } = s {
            if args.is_empty()
                && matches!(
                    name.as_str(),
                    "T" | "U" | "V" | "K" | "E" | "R" | "Cmp" | "Pred" | "Alloc"
                )
            {
                return true;
            }
        }
        // reference binding to same type
        if let Ty::Reference(inner) = dst {
            return self.assignable(inner, src);
        }
        // Init list into vector etc.
        if matches!(s, Ty::Unknown) {
            return true;
        }
        false
    }

    pub(super) fn resolve_ast_type(&mut self, ty: &Type) -> Ty {
        match ty {
            Type::Builtin { kind, .. } => match kind {
                BuiltinType::Void => Ty::Void,
                BuiltinType::Bool => Ty::Bool,
                BuiltinType::Char | BuiltinType::UnsignedChar => Ty::Char,
                BuiltinType::Short | BuiltinType::Int => Ty::Int,
                BuiltinType::Long => Ty::Long,
                BuiltinType::LongLong => Ty::LongLong,
                BuiltinType::UnsignedShort | BuiltinType::UnsignedInt => Ty::UInt,
                BuiltinType::UnsignedLong => Ty::ULong,
                BuiltinType::UnsignedLongLong => Ty::ULongLong,
                BuiltinType::Float => Ty::Float,
                BuiltinType::Double => Ty::Double,
                BuiltinType::WcharT => Ty::Int,
                BuiltinType::Auto => Ty::Auto,
            },
            Type::Named { path, args, span } => {
                let name = path.segments.last().map(|s| s.name.as_str()).unwrap_or("?");
                match name {
                    "int64_t" | "long long" => return Ty::LongLong,
                    "uint64_t" | "size_t" => return Ty::ULongLong,
                    "int32_t" => return Ty::Int,
                    "uint32_t" => return Ty::UInt,
                    _ => {}
                }
                // `using Parent = vector<int>;` — expand aliases.
                if args.is_empty() {
                    if let Some(aliased) = self.type_aliases.get(name).cloned() {
                        return self.resolve_ast_type(&aliased);
                    }
                    if let Some(sym) = self.symbols.lookup(name) {
                        if matches!(sym.kind, SymbolKind::Class) {
                            return sym.ty.clone();
                        }
                    }
                }
                // Unknown type name still allowed if looks like STL seed or class
                if self.symbols.lookup(name).is_none()
                    && path.segments.len() == 1
                    && !matches!(name, "std")
                {
                    // Soft: allow undeclared type names as opaque Named (LeetCode often omits includes)
                    let _ = span;
                }
                Ty::Named {
                    name: name.into(),
                    args: args.iter().map(|a| self.resolve_ast_type(a)).collect(),
                }
            }
            Type::Pointer { inner, .. } => Ty::Pointer(Box::new(self.resolve_ast_type(inner))),
            Type::Reference { inner, .. } => Ty::Reference(Box::new(self.resolve_ast_type(inner))),
            Type::Const { inner, .. } => Ty::Const(Box::new(self.resolve_ast_type(inner))),
        }
    }
}

fn ty_contains_auto(ty: &Ty) -> bool {
    match ty {
        Ty::Auto => true,
        Ty::Pointer(inner) | Ty::Reference(inner) | Ty::Const(inner) => ty_contains_auto(inner),
        _ => false,
    }
}

fn is_soft_operand(ty: &Ty) -> bool {
    match ty.strip_cv_ref() {
        Ty::Auto | Ty::Unknown => true,
        Ty::Named { name, args }
            if args.is_empty()
                && matches!(
                    name.as_str(),
                    "T" | "U" | "V" | "K" | "E" | "R" | "Cmp" | "Pred" | "Alloc"
                ) =>
        {
            true
        }
        _ => false,
    }
}
