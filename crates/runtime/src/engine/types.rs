use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn eval_binary(
        &mut self,
        op: BinaryOp,
        l: &Value,
        r: &Value,
        span: Span,
    ) -> Result<Value> {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitXor | BitOr | Shl | Shr => {
                let a = l.as_int().map_err(RuntimeError::new)?;
                let b = r.as_int().map_err(RuntimeError::new)?;
                let n = match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a / b
                    }
                    Rem => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a % b
                    }
                    BitAnd => a & b,
                    BitXor => a ^ b,
                    BitOr => a | b,
                    Shl => a << b,
                    Shr => a >> b,
                    _ => unreachable!(),
                };
                Ok(Value::Int(n))
            }
            Lt | Gt | Le | Ge | Eq | Ne => {
                let result =
                    match (l, r, op) {
                        (Value::Ptr(a), Value::Ptr(b), Eq) => a == b,
                        (Value::Ptr(a), Value::Ptr(b), Ne) => a != b,
                        (Value::Ptr(a), Value::Nullptr, Eq)
                        | (Value::Nullptr, Value::Ptr(a), Eq) => *a == Address::Null,
                        (Value::Ptr(a), Value::Nullptr, Ne)
                        | (Value::Nullptr, Value::Ptr(a), Ne) => *a != Address::Null,
                        (Value::Nullptr, Value::Nullptr, Eq) => true,
                        (Value::Nullptr, Value::Nullptr, Ne) => false,
                        (Value::Int(a), Value::Int(b), _) => cmp_ord(op, a.cmp(b)),
                        (Value::Float(a), Value::Float(b), _) => {
                            cmp_ord(op, a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        }
                        _ => {
                            let a = l.as_int().map_err(RuntimeError::new)?;
                            let b = r.as_int().map_err(RuntimeError::new)?;
                            cmp_ord(op, a.cmp(&b))
                        }
                    };
                self.emit(Event::Compare {
                    op: format!("{op:?}"),
                    left: l.clone(),
                    right: r.clone(),
                    result,
                    span,
                });
                Ok(Value::Bool(result))
            }
            And => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    && r.as_bool().map_err(RuntimeError::new)?,
            )),
            Or => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    || r.as_bool().map_err(RuntimeError::new)?,
            )),
            Comma => Ok(r.clone()),
        }
    }

    pub(super) fn alloc_empty_named(&mut self, name: &str, span: Span) -> Result<ObjId> {
        let obj = Object::empty_named(name)
            .ok_or_else(|| RuntimeError::at(span, format!("cannot default-construct `{name}`")))?;
        let kind = obj.kind_name().to_string();
        let id = self.heap.alloc(obj);
        self.emit_alloc(id, kind, span);
        Ok(id)
    }

    /// Allocate a user-class instance with defaulted fields from the class definition.
    pub(super) fn alloc_class_instance(&mut self, name: &str, span: Span) -> Result<ObjId> {
        let mut fields = HashMap::new();
        let field_decls: Vec<(String, Type, Option<InitDeclarator>)> =
            if let Some(c) = self.classes.get(name) {
                c.members
                    .iter()
                    .filter_map(|m| match m {
                        Member::Field(d) => Some(d),
                        _ => None,
                    })
                    .flat_map(|d| {
                        d.declarators
                            .iter()
                            .map(|dec| (dec.name.name.clone(), d.ty.clone(), Some(dec.clone())))
                    })
                    .collect()
            } else {
                vec![]
            };
        for (fname, ty, dec) in field_decls {
            let ty = self.resolve_type(&ty).clone();
            let v = if let Some(dec) = dec {
                // Field with in-class initializer, else default.
                if dec.init.is_some() {
                    self.eval_decl_init(&ty, &dec)?
                } else {
                    self.default_value_for_type(&ty)?
                }
            } else {
                self.default_value_for_type(&ty)?
            };
            fields.insert(fname, v);
        }
        let id = self.heap.alloc(Object::Class {
            name: name.into(),
            fields,
        });
        self.emit_alloc(id, name, span);
        Ok(id)
    }

    pub(super) fn resolve_type<'a>(&'a self, ty: &'a Type) -> &'a Type {
        let mut cur = ty;
        for _ in 0..8 {
            let name = match cur {
                Type::Named { path, args, .. } if args.is_empty() => {
                    path.segments.last().map(|s| s.name.as_str())
                }
                Type::Const { inner, .. } => {
                    cur = inner;
                    continue;
                }
                _ => None,
            };
            let Some(n) = name else {
                return cur;
            };
            if let Some(aliased) = self.type_aliases.get(n) {
                cur = aliased;
                continue;
            }
            return cur;
        }
        cur
    }

    pub(super) fn default_value_for_type(&mut self, ty: &Type) -> Result<Value> {
        let ty = self.resolve_type(ty).clone();
        match &ty {
            Type::Builtin { kind, .. } => Ok(match kind {
                BuiltinType::Bool => Value::Bool(false),
                BuiltinType::Float | BuiltinType::Double => Value::Float(0.0),
                BuiltinType::Void => Value::Void,
                BuiltinType::Char | BuiltinType::UnsignedChar => Value::Char('\0'),
                _ => Value::Int(0),
            }),
            Type::Named { path, args, .. } => {
                let name = path.segments.last().map(|s| s.name.as_str()).unwrap_or("");
                if name == "array" {
                    let n = args
                        .get(1)
                        .and_then(|t| match t {
                            Type::Named { path, .. } => {
                                path.segments.last()?.name.parse::<usize>().ok()
                            }
                            _ => None,
                        })
                        .unwrap_or(0);
                    let fill = if let Some(et) = args.first() {
                        self.default_value_for_type(et)?
                    } else {
                        Value::Int(0)
                    };
                    let elems = vec![fill; n];
                    let id = self.heap.alloc(Object::Array { elems, n });
                    self.emit_alloc(id, "array", path.span);
                    return Ok(Value::Object(id));
                }
                if let Some(obj) = Object::empty_named(name) {
                    let kind = obj.kind_name().to_string();
                    let id = self.heap.alloc(obj);
                    // Remember V for map<K,V> / unordered_map<K,V> so operator[] can default-insert.
                    if (kind == "map" || kind == "unordered_map") && args.len() >= 2 {
                        self.map_value_tys.insert(id, args[1].clone());
                    }
                    self.emit_alloc(id, kind, path.span);
                    return Ok(Value::Object(id));
                }
                if self.classes.contains_key(name) {
                    let id = self.alloc_class_instance(name, path.span)?;
                    let ctor = format!("{name}::{name}");
                    if self.functions.contains_key(&ctor) {
                        // Default-construct via zero-arg ctor when present (`Trie trie;`).
                        self.call_fn(&ctor, &[], Some(Value::Object(id)))?;
                    }
                    return Ok(Value::Object(id));
                }
                Ok(Value::Int(0))
            }
            Type::Pointer { .. } => Ok(Value::Nullptr),
            Type::Reference { inner, .. } => self.default_value_for_type(inner),
            Type::Const { inner, .. } => self.default_value_for_type(inner),
        }
    }

    /// C++ `map::operator[]`: return existing value, or default-construct mapped type and insert.
    pub(super) fn map_index_get_or_insert(
        &mut self,
        map_id: ObjId,
        key: &MapKey,
        span: Span,
    ) -> Result<Value> {
        let existing = match self.heap.get(map_id) {
            Some(Object::Map(m)) => m.get(key).cloned(),
            Some(Object::UnorderedMap(m)) => m.get(key).cloned(),
            _ => return Err(RuntimeError::at(span, "not a map")),
        };
        if let Some(v) = existing {
            return Ok(v);
        }

        let mapped_ty = self
            .map_value_tys
            .get(&map_id)
            .cloned()
            .unwrap_or(Type::Builtin {
                kind: BuiltinType::Int,
                span,
            });
        let def = self.default_value_for_type(&mapped_ty)?;
        match self.heap.get_mut(map_id) {
            Some(Object::Map(m)) => {
                m.insert(key.clone(), def.clone());
            }
            Some(Object::UnorderedMap(m)) => {
                m.insert(key.clone(), def.clone());
            }
            _ => return Err(RuntimeError::at(span, "not a map")),
        }
        self.emit(Event::Write {
            slot: Slot::MapEntry {
                obj: map_id,
                key: key.to_string(),
            },
            old: None,
            value: def.clone(),
            span,
        });
        self.emit(Event::ContainerMod {
            container: Value::Object(map_id),
            kind: "map_default_insert".into(),
            index: None,
            key: Some(key.to_value()),
            old: None,
            value: Some(def.clone()),
            elems: vec![],
            span,
        });
        Ok(def)
    }
}

fn cmp_ord(op: BinaryOp, o: std::cmp::Ordering) -> bool {
    use BinaryOp::*;
    match op {
        Lt => o.is_lt(),
        Gt => o.is_gt(),
        Le => o.is_le(),
        Ge => o.is_ge(),
        Eq => o.is_eq(),
        Ne => o.is_ne(),
        _ => false,
    }
}
