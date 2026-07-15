use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt;

/// Hashable / ordered key for map & set (LeetCode subset).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapKey {
    Int(i64),
    Bool(bool),
    Char(char),
    Str(String),
}

impl MapKey {
    pub fn from_value(v: &Value, string_of: impl FnOnce(u64) -> Option<String>) -> Result<Self, String> {
        match v {
            Value::Int(i) => Ok(MapKey::Int(*i)),
            Value::Bool(b) => Ok(MapKey::Bool(*b)),
            Value::Char(c) => Ok(MapKey::Char(*c)),
            Value::Object(id) => {
                if let Some(s) = string_of(*id) {
                    Ok(MapKey::Str(s))
                } else {
                    Err("map/set key must be int/bool/char/string".into())
                }
            }
            other => Err(format!("unsupported map/set key `{other}`")),
        }
    }
}

impl fmt::Display for MapKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapKey::Int(i) => write!(f, "{i}"),
            MapKey::Bool(b) => write!(f, "{b}"),
            MapKey::Char(c) => write!(f, "{c:?}"),
            MapKey::Str(s) => write!(f, "{s:?}"),
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
    Map(BTreeMap<MapKey, Value>),
    UnorderedMap(HashMap<MapKey, Value>),
    Set(BTreeSet<MapKey>),
    UnorderedSet(HashSet<MapKey>),
    Stack(Vec<Value>),
    Queue(VecDeque<Value>),
    /// Max-heap of integers (LeetCode default).
    PriorityQueue(BinaryHeap<i64>),
    Class {
        name: String,
        fields: HashMap<String, Value>,
    },
}

impl Object {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Object::Vector(_) => "vector",
            Object::Pair { .. } => "pair",
            Object::String(_) => "string",
            Object::Map(_) => "map",
            Object::UnorderedMap(_) => "unordered_map",
            Object::Set(_) => "set",
            Object::UnorderedSet(_) => "unordered_set",
            Object::Stack(_) => "stack",
            Object::Queue(_) => "queue",
            Object::PriorityQueue(_) => "priority_queue",
            Object::Class { .. } => "class",
        }
    }

    pub fn empty_named(name: &str) -> Option<Self> {
        Some(match name {
            "vector" => Object::Vector(vec![]),
            "string" => Object::String(String::new()),
            "pair" => Object::Pair {
                first: Value::Int(0),
                second: Value::Int(0),
            },
            "map" => Object::Map(BTreeMap::new()),
            "unordered_map" => Object::UnorderedMap(HashMap::new()),
            "set" => Object::Set(BTreeSet::new()),
            "unordered_set" => Object::UnorderedSet(HashSet::new()),
            "stack" => Object::Stack(vec![]),
            "queue" => Object::Queue(VecDeque::new()),
            "priority_queue" => Object::PriorityQueue(BinaryHeap::new()),
            _ => return None,
        })
    }
}

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

    pub fn string_value(&self, id: ObjId) -> Option<String> {
        match self.get(id) {
            Some(Object::String(s)) => Some(s.clone()),
            _ => None,
        }
    }
}
