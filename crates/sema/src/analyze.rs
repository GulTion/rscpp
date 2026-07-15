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
    }

    fn analyze_tu(&mut self, tu: &TranslationUnit) {
        // Pass 1: declare classes and functions (signatures).
        for item in &tu.items {
            self.declare_item(item);
        }
        // Pass 2: check bodies.
        for item in &tu.items {
            self.check_item(item);
        }
    }

    fn declare_item(&mut self, item: &Item) {
        match item {
            Item::UsingNamespace { .. } => {}
            Item::Class(c) => {
                let name = c.name.name.clone();
                if let Err(prev) = self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: Ty::named(&name, vec![]),
                    kind: SymbolKind::Class,
                }) {
                    if prev.kind != SymbolKind::Class {
                        self.err(c.name.span, format!("redefinition of `{}`", c.name.name));
                    }
                }
                // Register methods as ClassName::method in a flat map via qualified later;
                // store fields/methods on a side table keyed by class name.
                self.register_class_members(c);
            }
            Item::Function(f) => {
                let ret = self.resolve_ast_type(&f.return_type);
                let params: Vec<Ty> = f.params.iter().map(|p| self.resolve_ast_type(&p.ty)).collect();
                let ty = Ty::Function {
                    ret: Box::new(ret),
                    params,
                };
                if let Err(_) = self.symbols.define(Symbol {
                    name: f.name.name.clone(),
                    ty,
                    kind: SymbolKind::Func,
                }) {
                    self.err(f.name.span, format!("redefinition of `{}`", f.name.name));
                }
            }
            Item::Decl(d) => {
                let ty = self.resolve_ast_type(&d.ty);
                for decl in &d.declarators {
                    let mut t = ty.clone();
                    for p in &decl.ptrs {
                        t = match p {
                            PtrKind::Pointer => Ty::Pointer(Box::new(t)),
                            PtrKind::Reference => Ty::Reference(Box::new(t)),
                        };
                    }
                    if let Err(_) = self.symbols.define(Symbol {
                        name: decl.name.name.clone(),
                        ty: t,
                        kind: SymbolKind::Var,
                    }) {
                        self.err(
                            decl.name.span,
                            format!("redefinition of `{}`", decl.name.name),
                        );
                    }
                }
            }
        }
    }

    /// class_name -> (field/method name -> Ty)
    // stored in symbols as "Class::name" for ponytail simplicity
    fn register_class_members(&mut self, c: &ClassDef) {
        let class = &c.name.name;
        for m in &c.members {
            match m {
                Member::Access(_) => {}
                Member::Field(d) => {
                    let ty = self.resolve_ast_type(&d.ty);
                    for decl in &d.declarators {
                        let qname = format!("{class}::{}", decl.name.name);
                        self.symbols.redefine(Symbol {
                            name: qname,
                            ty: ty.clone(),
                            kind: SymbolKind::Field,
                        });
                    }
                }
                Member::Function(f) => {
                    let ret = self.resolve_ast_type(&f.return_type);
                    let params: Vec<Ty> =
                        f.params.iter().map(|p| self.resolve_ast_type(&p.ty)).collect();
                    let qname = format!("{class}::{}", f.name.name);
                    self.symbols.redefine(Symbol {
                        name: qname,
                        ty: Ty::Function {
                            ret: Box::new(ret),
                            params,
                        },
                        kind: SymbolKind::Func,
                    });
                }
            }
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::UsingNamespace { .. } | Item::Decl(_) => {}
            Item::Function(f) => self.check_function(f, None),
            Item::Class(c) => {
                self.current_class = Some(c.name.name.clone());
                for m in &c.members {
                    if let Member::Function(f) = m {
                        self.check_function(f, Some(&c.name.name));
                    }
                }
                self.current_class = None;
            }
        }
    }

    fn check_function(&mut self, f: &FunctionDef, class: Option<&str>) {
        self.symbols.push();
        // `this` in methods
        if let Some(c) = class {
            let _ = self.symbols.define(Symbol {
                name: "this".into(),
                ty: Ty::Pointer(Box::new(Ty::named(c, vec![]))),
                kind: SymbolKind::Var,
            });
        }
        for p in &f.params {
            let ty = self.resolve_ast_type(&p.ty);
            if let Some(name) = &p.name {
                if let Err(_) = self.symbols.define(Symbol {
                    name: name.name.clone(),
                    ty,
                    kind: SymbolKind::Var,
                }) {
                    self.err(name.span, format!("redefinition of parameter `{}`", name.name));
                }
            }
        }
        self.check_block(&f.body);
        self.symbols.pop();
    }

    fn check_block(&mut self, block: &Block) {
        self.symbols.push();
        for s in &block.stmts {
            self.check_stmt(s);
        }
        self.symbols.pop();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(b) => self.check_block(b),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let _ = self.check_expr(cond);
                self.check_stmt(then_branch);
                if let Some(e) = else_branch {
                    self.check_stmt(e);
                }
            }
            Stmt::While { cond, body, .. } | Stmt::DoWhile { cond, body, .. } => {
                let _ = self.check_expr(cond);
                self.check_stmt(body);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.symbols.push();
                match init {
                    Some(ForInit::Decl(d)) => self.check_decl(d),
                    Some(ForInit::Expr(e)) => {
                        let _ = self.check_expr(e);
                    }
                    None => {}
                }
                if let Some(c) = cond {
                    let _ = self.check_expr(c);
                }
                if let Some(s) = step {
                    let _ = self.check_expr(s);
                }
                self.check_stmt(body);
                self.symbols.pop();
            }
            Stmt::Return { value, span } => {
                if let Some(v) = value {
                    let _ = self.check_expr(v);
                }
                let _ = span;
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Expr { expr, .. } => {
                let _ = self.check_expr(expr);
            }
            Stmt::Decl(d) => self.check_decl(d),
        }
    }

    fn check_decl(&mut self, d: &Decl) {
        let base = self.resolve_ast_type(&d.ty);
        for decl in &d.declarators {
            let mut ty = base.clone();
            for p in &decl.ptrs {
                ty = match p {
                    PtrKind::Pointer => Ty::Pointer(Box::new(ty)),
                    PtrKind::Reference => Ty::Reference(Box::new(ty)),
                };
            }
            if let Some(init) = &decl.init {
                let it = self.check_expr(init);
                if !self.assignable(&ty, &it) && it != Ty::Error && it != Ty::Unknown {
                    self.err(
                        decl.span,
                        format!("cannot initialize `{ty}` with `{it}`"),
                    );
                }
            }
            if let Err(_) = self.symbols.define(Symbol {
                name: decl.name.name.clone(),
                ty,
                kind: SymbolKind::Var,
            }) {
                self.err(
                    decl.name.span,
                    format!("redefinition of `{}`", decl.name.name),
                );
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Ty {
        match expr {
            Expr::IntLit { .. } => Ty::Int,
            Expr::FloatLit { .. } => Ty::Double,
            Expr::CharLit { .. } => Ty::Char,
            Expr::StringLit { .. } => Ty::named("string", vec![]),
            Expr::BoolLit { .. } => Ty::Bool,
            Expr::Nullptr { .. } => Ty::Pointer(Box::new(Ty::Void)),
            Expr::Name(path) => self.lookup_path(path),
            Expr::Unary { op, expr, span } => {
                let t = self.check_expr(expr);
                self.check_unary(*op, &t, *span)
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                self.check_binary(*op, &lt, &rt, *span)
            }
            Expr::Assign {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                if *op == AssignOp::Assign {
                    if !self.assignable(&lt, &rt) && rt != Ty::Error && lt != Ty::Error {
                        self.err(*span, format!("cannot assign `{rt}` to `{lt}`"));
                    }
                    lt
                } else {
                    // compound assign: require numerics mostly
                    if !lt.is_numeric() || !rt.is_numeric() {
                        if lt != Ty::Error && rt != Ty::Error {
                            self.err(*span, format!("invalid compound assignment on `{lt}` and `{rt}`"));
                        }
                    }
                    lt
                }
            }
            Expr::Call { callee, args, span } => {
                let ct = self.check_expr(callee);
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a)).collect();
                match ct.strip_cv_ref() {
                    Ty::Function { ret, params } => {
                        if params.len() != arg_tys.len() {
                            self.err(
                                *span,
                                format!(
                                    "call expects {} argument(s), got {}",
                                    params.len(),
                                    arg_tys.len()
                                ),
                            );
                        } else {
                            for (p, a) in params.iter().zip(arg_tys.iter()) {
                                if !self.assignable(p, a) && *a != Ty::Error {
                                    self.err(*span, format!("argument type `{a}` not compatible with `{p}`"));
                                }
                            }
                        }
                        *ret.clone()
                    }
                    Ty::Named { name, args: targs } => {
                        // Construct temporary: Type(args) — treat as returning that type.
                        let _ = (name, targs, arg_tys);
                        ct.strip_cv_ref().clone()
                    }
                    Ty::Unknown | Ty::Error => Ty::Error,
                    other => {
                        self.err(*span, format!("cannot call value of type `{other}`"));
                        Ty::Error
                    }
                }
            }
            Expr::Index { base, index, span } => {
                let bt = self.check_expr(base);
                let it = self.check_expr(index);
                if !it.is_integral() && it != Ty::Error && it != Ty::Unknown {
                    self.err(*span, format!("array index must be integral, got `{it}`"));
                }
                self.elem_type(&bt, *span)
            }
            Expr::Member {
                base,
                field,
                arrow,
                span,
            } => {
                let mut bt = self.check_expr(base);
                if *arrow {
                    match bt.strip_cv_ref() {
                        Ty::Pointer(inner) => bt = *inner.clone(),
                        Ty::Error | Ty::Unknown => {}
                        other => {
                            self.err(*span, format!("base of `->` has type `{other}`, not a pointer"));
                        }
                    }
                }
                self.lookup_member(&bt, &field.name, field.span)
            }
            Expr::Cast { ty, expr, .. } => {
                let _ = self.check_expr(expr);
                self.resolve_ast_type(ty)
            }
            Expr::InitList { elems, .. } => {
                for e in elems {
                    let _ = self.check_expr(e);
                }
                // brace init — unknown concrete type without context
                Ty::Unknown
            }
        }
    }

    fn lookup_path(&mut self, path: &Path) -> Ty {
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

    fn lookup_member(&mut self, base: &Ty, field: &str, span: Span) -> Ty {
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

    fn elem_type(&mut self, base: &Ty, span: Span) -> Ty {
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
            Ty::Unknown => Ty::Unknown,
            Ty::Error => Ty::Error,
            other => {
                self.err(span, format!("type `{other}` is not subscriptable"));
                Ty::Error
            }
        }
    }

    fn check_unary(&mut self, op: UnaryOp, t: &Ty, span: Span) -> Ty {
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

    fn check_binary(&mut self, op: BinaryOp, lt: &Ty, rt: &Ty, span: Span) -> Ty {
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

    fn assignable(&self, dst: &Ty, src: &Ty) -> bool {
        if matches!(dst, Ty::Unknown | Ty::Error | Ty::Auto) || matches!(src, Ty::Unknown | Ty::Error) {
            return true;
        }
        let d = dst.strip_cv_ref();
        let s = src.strip_cv_ref();
        if d == s {
            return true;
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

    fn resolve_ast_type(&mut self, ty: &Type) -> Ty {
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

fn stl_member(name: &str, args: &[Ty], field: &str) -> Option<Ty> {
    match (name, field) {
        ("vector", "size") | ("string", "size") | ("map", "size") | ("unordered_map", "size")
        | ("set", "size") | ("queue", "size") | ("stack", "size") => Some(Ty::Function {
            ret: Box::new(Ty::UInt),
            params: vec![],
        }),
        ("vector", "empty") | ("string", "empty") | ("map", "empty") | ("queue", "empty")
        | ("stack", "empty") => Some(Ty::Function {
            ret: Box::new(Ty::Bool),
            params: vec![],
        }),
        ("vector", "push_back") => {
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
        ("vector", "begin") | ("vector", "end") => Some(Ty::Unknown),
        ("map", "insert") | ("unordered_map", "insert") | ("set", "insert") => Some(Ty::Function {
            ret: Box::new(Ty::Unknown),
            params: vec![Ty::Unknown],
        }),
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
        ("queue", "front") | ("stack", "top") => {
            let elem = args.first().cloned().unwrap_or(Ty::Unknown);
            Some(Ty::Reference(Box::new(elem)))
        }
        ("queue", "pop") | ("stack", "pop") => Some(Ty::Function {
            ret: Box::new(Ty::Void),
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
