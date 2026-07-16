use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
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
                    call_id: self.current_call_id(),
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
        if Self::type_is_ref(ty) {
            let init = decl
                .init
                .as_ref()
                .ok_or_else(|| RuntimeError::at(decl.span, "reference must be initialized"))?;
            let (_v, lv) = self.eval_expr_lv(init)?;
            let lv = lv.ok_or_else(|| RuntimeError::at(decl.span, "cannot bind ref to rvalue"))?;
            let addr = self.lvalue_to_address(&lv)?;
            return Ok(Value::Ref(addr));
        }
        if let Some(init) = &decl.init {
            // `pair<K,V> p = {a, b};`
            if let Expr::InitList { elems, span } = init {
                if let Type::Named { path, .. } = ty {
                    let tname = path
                        .segments
                        .last()
                        .map(|s| s.name.as_str())
                        .unwrap_or("");
                    if tname == "pair" && elems.len() == 2 {
                        let first = self.eval_expr(&elems[0])?;
                        let second = self.eval_expr(&elems[1])?;
                        let id = self.heap.alloc(Object::Pair { first, second });
                        self.emit_alloc(id, "pair", *span);
                        return Ok(Value::Object(id));
                    }
                }
            }
            // `vector<T> v(n);` / `vector<T> v(n, fill)`
            if let Expr::Call { args, span, .. } = init {
                if let Type::Named { path, args: targs, .. } = ty {
                    let tname = path
                        .segments
                        .last()
                        .map(|s| s.name.as_str())
                        .unwrap_or("");
                    if tname == "vector" && (args.len() == 1 || args.len() == 2) {
                        let n = self.eval_expr(&args[0])?.as_int().map_err(RuntimeError::new)? as usize;
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
                }
            }
            let v = self.eval_expr(init)?;
            if Self::type_is_ptr(ty) {
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
            return Ok(v);
        }
        if Self::type_is_ptr(ty) {
            return Ok(Value::Ptr(Address::Null));
        }
        self.default_value_for_type(ty)
    }

    pub(super) fn dealloc_owned_locals(&mut self, locals: &HashMap<String, Value>, keep: Option<&Value>) {
        let keep_id = match keep {
            Some(Value::Object(id)) => Some(*id),
            Some(Value::Ref(Address::Heap(id)) | Value::Ptr(Address::Heap(id))) => Some(*id),
            _ => None,
        };
        for (_name, v) in locals {
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
                        call_id: self.current_call_id(),
                        id: *id,
                        span: Span::new(0, 0),
                    });
                }
            }
        }
    }

    pub(super) fn value_mentions_obj(&self, id: ObjId) -> bool {
        let check = |v: &Value| matches!(v, Value::Object(x) if *x == id)
            || matches!(v, Value::Ptr(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id)
            || matches!(v, Value::Ref(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id);
        for f in &self.stack {
            if f.locals.values().any(check) {
                return true;
            }
        }
        self.globals.values().any(check)
}
}
