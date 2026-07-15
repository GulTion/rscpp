use std::collections::HashMap;
use std::fmt;

/// Heap object identity.
pub type ObjId = u64;

/// Runtime values (by-value primitives + heap handles).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Void,
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(char),
    Nullptr,
    /// Handle into the heap.
    Object(ObjId),
}

impl Value {
    pub fn as_bool(&self) -> Result<bool, String> {
        Ok(match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Char(c) => *c != '\0',
            Value::Nullptr => false,
            Value::Object(_) => true,
            Value::Void => return Err("void is not truthy".into()),
        })
    }

    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
            Value::Char(c) => Ok(*c as i64),
            Value::Float(f) => Ok(*f as i64),
            other => Err(format!("cannot convert `{other}` to int")),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Void => write!(f, "void"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Char(c) => write!(f, "{c:?}"),
            Value::Nullptr => write!(f, "nullptr"),
            Value::Object(id) => write!(f, "obj#{id}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Object {
    Vector(Vec<Value>),
    Pair {
        first: Value,
        second: Value,
    },
    String(String),
    Class {
        name: String,
        fields: HashMap<String, Value>,
    },
}

#[derive(Debug, Default)]
pub struct Heap {
    next_id: ObjId,
    objects: HashMap<ObjId, Object>,
}

impl Heap {
    pub fn alloc(&mut self, obj: Object) -> ObjId {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(id, obj);
        id
    }

    pub fn get(&self, id: ObjId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: ObjId) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    pub fn free(&mut self, id: ObjId) -> Option<Object> {
        self.objects.remove(&id)
    }
}
