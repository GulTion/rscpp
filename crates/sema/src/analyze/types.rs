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
            self.err(id.span, format!("use of undeclared identifier `{}`", id.name));
            return Ty::Error;
        }
        // std::vector / Namespace::name — treat last segment; if std::X just Named(X)
        let last = path.segments.last().unwrap();
        if path.segments.len() == 2 && path.segments[0].name == "std" {
            if let Some(sym) = self.symbols.lookup(&last.name) {
                return sym.ty.clone();
            }
            return Ty::named(&last.name, vec![]);
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
                            self.err(span, format!("argument type `{a}` not compatible with `{p}`"));
                        }
                    }
                }
                *ret.clone()
            }
            Ty::Named { name, args: targs } => {
                let _ = (name, targs, arg_tys);
                ct.strip_cv_ref().clone()
            }
            Ty::Unknown | Ty::Error | Ty::Auto => Ty::Unknown,
            other => {
                self.err(span, format!("cannot call value of type `{other}`"));
                Ty::Error
            }
        }
    }

    pub(super) fn lookup_member(&mut self, base: &Ty, field: &str, span: Span) -> Ty {
        let base = base.strip_cv_ref();
        match base {
            Ty::Named { name, args } => {
                // User class field/method
                let q = format!("{name}::{field}");
                if let Some(sym) = self.symbols.lookup(&q) {
                    return sym.ty.clone();
                }
                if let Some(ty) = stl_member(name, args, field) {
                    return ty;
                }
                self.err(span, format!("no member `{field}` on type `{base}`"));
                Ty::Error
            }
            Ty::Unknown => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("member access on non-class type `{other}`"));
                Ty::Error
            }
        }
    }

    pub(super) fn elem_type(&mut self, base: &Ty, span: Span) -> Ty {
        match base.strip_cv_ref() {
            Ty::Named { name, args } if name == "vector" || name == "string" => {
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
            Ty::Pointer(inner) => *inner.clone(),
            Ty::Unknown | Ty::Auto => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("type `{other}` is not subscriptable"));
                Ty::Error
            }
        }
    }

    pub(super) fn check_unary(&mut self, op: UnaryOp, t: &Ty, span: Span) -> Ty {
        match op {
            UnaryOp::Plus | UnaryOp::Minus | UnaryOp::PreInc | UnaryOp::PreDec | UnaryOp::PostInc
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
                Ty::Error | Ty::Unknown => Ty::Error,
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
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitXor | BitOr | Shl | Shr => {
                if (!lt.is_numeric() || !rt.is_numeric())
                    && *lt != Ty::Error
                    && *rt != Ty::Error
                    && *lt != Ty::Unknown
                    && *rt != Ty::Unknown
                {
                    // allow string + for later; for now error
                    self.err(span, format!("invalid operands `{lt}` and `{rt}` to binary op"));
                }
                // promote roughly: if either floating → double else int
                if matches!(lt.strip_cv_ref(), Ty::Float | Ty::Double)
                    || matches!(rt.strip_cv_ref(), Ty::Float | Ty::Double)
                {
                    Ty::Double
                } else {
                    Ty::Int
                }
            }
            Lt | Gt | Le | Ge | Eq | Ne => Ty::Bool,
            And | Or => Ty::Bool,
        }
    }

    pub(super) fn assignable(&self, dst: &Ty, src: &Ty) -> bool {
        if matches!(dst, Ty::Unknown | Ty::Error | Ty::Auto) || matches!(src, Ty::Unknown | Ty::Error) {
            return true;
        }
        let d = dst.strip_cv_ref();
        let s = src.strip_cv_ref();
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
        // pointer: T* <- nullptr (void*)
        if let (Ty::Pointer(_), Ty::Pointer(inner)) = (d, s) {
            if matches!(inner.as_ref(), Ty::Void) {
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
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.as_str())
                    .unwrap_or("?");
                match name {
                    "int64_t" | "long long" => return Ty::LongLong,
                    "uint64_t" | "size_t" => return Ty::ULongLong,
                    "int32_t" => return Ty::Int,
                    "uint32_t" => return Ty::UInt,
                    _ => {}
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
