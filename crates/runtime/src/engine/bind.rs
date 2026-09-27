use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn type_is_ref(ty: &Type) -> bool {
        match ty {
            Type::Reference { .. } => true,
            Type::Const { inner, .. } => Self::type_is_ref(inner),
            _ => false,
        }
    }

    pub(super) fn type_is_ptr(ty: &Type) -> bool {
        match ty {
            Type::Pointer { .. } => true,
            Type::Const { inner, .. } => Self::type_is_ptr(inner),
            Type::Reference { inner, .. } => Self::type_is_ptr(inner),
            _ => false,
        }
    }

    /// Apply declarator `*`/`&` layers onto the decl base type (C++ grammar).
    pub(super) fn decl_type(ty: &Type, decl: &InitDeclarator) -> Type {
        let mut t = ty.clone();
        for p in &decl.ptrs {
            let span = t.span();
            t = match p {
                PtrKind::Pointer => Type::Pointer {
                    inner: Box::new(t),
                    span,
                },
                PtrKind::Reference => Type::Reference {
                    inner: Box::new(t),
                    span,
                },
            };
        }
        t
    }

    pub(super) fn bind_param_value(
        &mut self,
        ty: &Type,
        arg: &Value,
        name: &str,
        span: Span,
    ) -> Result<Value> {
        if Self::type_is_ref(ty) {
            let addr = match arg {
                Value::Object(id) => Address::Heap(*id),
                Value::Ref(a) | Value::Ptr(a) => a.clone(),
                Value::Nullptr => Address::Null,
                other => {
                    return Err(RuntimeError::at(
                        span,
                        format!("cannot bind reference parameter to `{other}`"),
                    ))
                }
            };
            if let Some(slot) = Self::address_to_slot(&addr) {
                self.emit(Event::RefBind {
                    name: name.to_string(),
                    target: slot,
                    span,
                });
            }
            return Ok(Value::Ref(addr));
        }
        Ok(arg.clone())
    }

    pub(super) fn eval_decl_init(&mut self, ty: &Type, decl: &InitDeclarator) -> Result<Value> {
        let ty = Self::decl_type(ty, decl);
        if Self::type_is_ref(&ty) {
            let init = decl
                .init
                .as_ref()
                .ok_or_else(|| RuntimeError::at(decl.span, "reference must be initialized"))?;
            let (v, lv) = self.eval_expr_lv(init)?;
            if let Some(lv) = lv {
                let addr = self.lvalue_to_address(&lv)?;
                return Ok(Value::Ref(addr));
            }
            // `const T& x = temporary;` — own the value (LeetCode-common).
            return Ok(v);
        }
        if let Some(init) = &decl.init {
            // `pair<K,V> p = {a, b};`
            if let Expr::InitList { elems, span } = init {
                if let Type::Named { path, .. } = &ty {
                    let tname = path.segments.last().map(|s| s.name.as_str()).unwrap_or("");
                    if tname == "pair" && elems.len() == 2 {
                        let first = self.eval_expr(&elems[0])?;
                        let second = self.eval_expr(&elems[1])?;
                        let id = self.heap.alloc(Object::Pair { first, second });
                        self.emit_alloc(id, "pair", *span);
                        return Ok(Value::Object(id));
                    }
                    // `map` / `unordered_map` brace init: {{k,v}, ...}
                    if tname == "map" || tname == "unordered_map" {
                        let id = self.alloc_empty_named(tname, *span)?;
                        if let Type::Named { args: targs, .. } = &ty {
                            if targs.len() >= 2 {
                                self.map_value_tys.insert(id, targs[1].clone());
                            }
                        }
                        for e in elems {
                            let (k, v) = match e {
                                Expr::InitList { elems: kv, .. } if kv.len() == 2 => {
                                    (self.eval_expr(&kv[0])?, self.eval_expr(&kv[1])?)
                                }
                                other => {
                                    let pv = self.eval_expr(other)?;
                                    match &pv {
                                        Value::Object(pid) => match self.heap.get(*pid) {
                                            Some(Object::Pair { first, second }) => {
                                                (first.clone(), second.clone())
                                            }
                                            _ => {
                                                return Err(RuntimeError::at(
                                                    *span,
                                                    "map init expects {key, value} pairs",
                                                ))
                                            }
                                        },
                                        _ => {
                                            return Err(RuntimeError::at(
                                                *span,
                                                "map init expects {key, value} pairs",
                                            ))
                                        }
                                    }
                                }
                            };
                            let key = self.value_to_key(&k)?;
                            match self.heap.get_mut(id) {
                                Some(Object::Map(m)) => {
                                    m.insert(key, v);
                                }
                                Some(Object::UnorderedMap(m)) => {
                                    m.insert(key, v);
                                }
                                _ => {}
                            }
                        }
                        return Ok(Value::Object(id));
                    }
                    // `set` / `unordered_set` brace init: {a, b, c}
                    if tname == "set" || tname == "unordered_set" {
                        let id = self.alloc_empty_named(tname, *span)?;
                        for e in elems {
                            let v = self.eval_expr(e)?;
                            let key = self.value_to_key(&v)?;
                            match self.heap.get_mut(id) {
                                Some(Object::Set(s)) => {
                                    s.insert(key);
                                }
                                Some(Object::UnorderedSet(s)) => {
                                    s.insert(key);
                                }
                                _ => {}
                            }
                        }
                        return Ok(Value::Object(id));
                    }
                }
            }
            // `vector<T> v(n);` / `vector<T> v(n, fill)` / set from iterators
            if let Expr::Call { args, span, .. } = init {
                if let Type::Named {
                    path, args: targs, ..
                } = &ty
                {
                    let tname = path.segments.last().map(|s| s.name.as_str()).unwrap_or("");
                    if tname == "bitset" {
                        let n = Self::nttp_usize(targs).unwrap_or(0);
                        let ulong = if args.is_empty() {
                            0u64
                        } else {
                            self.eval_expr(&args[0])?
                                .as_int()
                                .map_err(RuntimeError::new)? as u64
                        };
                        let id = self.heap.alloc(Self::bitset_from_ulong(n, ulong));
                        self.emit_alloc(id, "bitset", *span);
                        return Ok(Value::Object(id));
                    }
                    if tname == "vector" && (args.len() == 1 || args.len() == 2) {
                        // Prefer size ctor when first arg is an integer expression, not begin/end.
                        let first_is_range = matches!(
                            &args[0],
                            Expr::Call { callee, .. } if matches!(
                                callee.as_ref(),
                                Expr::Member { field, .. } if matches!(
                                    field.name.as_str(),
                                    "begin" | "cbegin" | "rbegin" | "crbegin"
                                )
                            ) || matches!(
                                callee.as_ref(),
                                Expr::Name(p) if matches!(
                                    p.segments.last().map(|s| s.name.as_str()),
                                    Some("begin" | "cbegin" | "rbegin" | "crbegin"
                                        | "std::begin" | "std::cbegin")
                                )
                            )
                        );
                        if !first_is_range {
                            let n = self
                                .eval_expr(&args[0])?
                                .as_int()
                                .map_err(RuntimeError::new)?
                                as usize;
                            let fill = if args.len() == 2 {
                                self.eval_expr(&args[1])?
                            } else if let Some(et) = targs.first() {
                                self.default_value_for_type(et)?
                            } else {
                                Value::Int(0)
                            };
                            let elems = vec![fill; n];
                            let id = self.heap.alloc(Object::Vector(elems));
                            self.emit_alloc(id, "vector", *span);
                            return Ok(Value::Object(id));
                        }
                        // `vector<T> r(v.begin(), v.end())` / `vector(v.rbegin(), v.rend())`
                        if args.len() == 2 {
                            let (vid, rev) =
                                self.resolve_sequence_range(&args[0], &args[1], *span)?;
                            let mut elems = match self.heap.get(vid) {
                                Some(Object::Vector(e)) => e.clone(),
                                _ => {
                                    return Err(RuntimeError::at(
                                        *span,
                                        "vector range ctor needs a vector",
                                    ))
                                }
                            };
                            if rev {
                                elems.reverse();
                            }
                            let id = self.heap.alloc(Object::Vector(elems));
                            self.emit_alloc(id, "vector", *span);
                            return Ok(Value::Object(id));
                        }
                    }
                    // `unordered_set<T> s(v.begin(), v.end())` / free begin/end
                    if (tname == "set" || tname == "unordered_set") && args.len() == 2 {
                        let vid = self.resolve_vector_range(&args[0], &args[1], *span)?;
                        let elems = match self.heap.get(vid) {
                            Some(Object::Vector(e)) => e.clone(),
                            _ => {
                                return Err(RuntimeError::at(
                                    *span,
                                    "set range ctor needs a vector",
                                ))
                            }
                        };
                        let id = self.alloc_empty_named(tname, *span)?;
                        for v in elems {
                            let key = self.value_to_key(&v)?;
                            match self.heap.get_mut(id) {
                                Some(Object::Set(s)) => {
                                    s.insert(key);
                                }
                                Some(Object::UnorderedSet(s)) => {
                                    s.insert(key);
                                }
                                _ => {}
                            }
                        }
                        return Ok(Value::Object(id));
                    }
                }
            }
            let v = self.eval_expr(init)?;
            if Self::type_is_ptr(&ty) {
                return Ok(match v {
                    Value::Ptr(a) => Value::Ptr(a),
                    Value::Nullptr => Value::Ptr(Address::Null),
                    Value::Object(id) => Value::Ptr(Address::Heap(id)),
                    Value::Ref(a) => Value::Ptr(a),
                    other => {
                        return Err(RuntimeError::at(
                            decl.span,
                            format!("cannot initialize pointer from `{other}`"),
                        ))
                    }
                });
            }
            // `bitset<N> b = 0;` / `bitset<N> b(val);`
            if let Type::Named {
                path, args: targs, ..
            } = &ty
            {
                let tname = path.segments.last().map(|s| s.name.as_str()).unwrap_or("");
                if tname == "bitset" {
                    let n = Self::nttp_usize(targs).unwrap_or(0);
                    let ulong = match &v {
                        Value::Int(i) => *i as u64,
                        Value::Bool(b) => {
                            if *b {
                                1
                            } else {
                                0
                            }
                        }
                        other => {
                            return Err(RuntimeError::at(
                                decl.span,
                                format!("bitset init expects integer, got `{other}`"),
                            ))
                        }
                    };
                    let id = self.heap.alloc(Self::bitset_from_ulong(n, ulong));
                    self.emit_alloc(id, "bitset", decl.span);
                    return Ok(Value::Object(id));
                }
            }
            return Ok(v);
        }
        if Self::type_is_ptr(&ty) {
            return Ok(Value::Ptr(Address::Null));
        }
        self.default_value_for_type(&ty)
    }

    pub(super) fn dealloc_owned_locals(
        &mut self,
        locals: &HashMap<String, Value>,
        keep: Option<&Value>,
    ) {
        let keep_id = match keep {
            Some(Value::Object(id)) => Some(*id),
            Some(Value::Ref(Address::Heap(id)) | Value::Ptr(Address::Heap(id))) => Some(*id),
            _ => None,
        };
        for (name, v) in locals {
            // Caller owns `this` (especially after ctors).
            if name == "this" {
                continue;
            }
            if let Value::Object(id) = v {
                if Some(*id) == keep_id {
                    continue;
                }
                // Only free if no other references remain — check stack/globals roughly
                if self.value_mentions_obj(*id) {
                    continue;
                }
                if self.heap.free(*id).is_some() {
                    self.emit(Event::Dealloc {
                        id: *id,
                        span: Span::new(0, 0),
                    });
                }
            }
        }
    }

    pub(super) fn value_mentions_obj(&self, id: ObjId) -> bool {
        let check = |v: &Value| {
            matches!(v, Value::Object(x) if *x == id)
                || matches!(v, Value::Ptr(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id)
                || matches!(v, Value::Ref(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id)
        };
        for f in &self.stack {
            if f.locals.values().any(check) {
                return true;
            }
        }
        self.globals.values().any(check)
    }
}
