use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt;

use serde::Serialize;

/// Hashable / ordered key for map & set (LeetCode subset).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum MapKey {
    Int(i64),
    Bool(bool),
    Char(char),
    Str(String),
}

impl MapKey {
    pub fn from_value(
        v: &Value,
        string_of: impl FnOnce(u64) -> Option<String>,
    ) -> Result<Self, String> {
        match v {
            Value::Int(i) => Ok(MapKey::Int(*i)),
            Value::Bool(b) => Ok(MapKey::Bool(*b)),
            Value::Char(c) => Ok(MapKey::Char(*c)),
            Value::Str(s) => Ok(MapKey::Str(s.clone())),
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

    pub fn to_value(&self) -> Value {
        match self {
            MapKey::Int(i) => Value::Int(*i),
            MapKey::Bool(b) => Value::Bool(*b),
            MapKey::Char(c) => Value::Char(*c),
            MapKey::Str(s) => Value::Str(s.clone()),
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
    /// `std::deque` — double-ended.
    Deque(VecDeque<Value>),
    /// `std::list` — vector-backed (no node semantics).
    List(Vec<Value>),
    /// `std::array` — fixed length `n`.
    Array {
        elems: Vec<Value>,
        n: usize,
    },
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
    /// Heap of integers. `min_heap` when Compare is `greater` (LeetCode subset).
    PriorityQueue {
        heap: BinaryHeap<i64>,
        min_heap: bool,
    },
    Class {
        name: String,
        fields: HashMap<String, Value>,
    },
    /// Lambda / closure: params + body + captured locals.
    Closure {
        params: Vec<String>,
        body: rscpp_ast::Block,
        captures: HashMap<String, Value>,
    },
    /// `std::greater` / `plus` / … function object (`<functional>`).
    Functor {
        kind: FunctorKind,
    },
}

/// Standard `<functional>` function-object kind (template args erased by parser).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctorKind {
    Plus,
    Minus,
    Multiplies,
    Divides,
    Modulus,
    Negate,
    EqualTo,
    NotEqualTo,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
}

impl Object {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Object::Vector(_) => "vector",
            Object::Deque(_) => "deque",
            Object::List(_) => "list",
            Object::Array { .. } => "array",
            Object::Pair { .. } => "pair",
            Object::String(_) => "string",
            Object::Map(_) => "map",
            Object::UnorderedMap(_) => "unordered_map",
            Object::Set(_) => "set",
            Object::UnorderedSet(_) => "unordered_set",
            Object::Stack(_) => "stack",
            Object::Queue(_) => "queue",
            Object::PriorityQueue { .. } => "priority_queue",
            Object::Class { .. } => "class",
            Object::Closure { .. } => "closure",
            Object::Functor { .. } => "functor",
        }
    }

    pub fn empty_named(name: &str) -> Option<Self> {
        Some(match name {
            "vector" => Object::Vector(vec![]),
            "deque" => Object::Deque(VecDeque::new()),
            "list" => Object::List(vec![]),
            "array" => Object::Array {
                elems: vec![],
                n: 0,
            },
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
            "priority_queue" => Object::PriorityQueue {
                heap: BinaryHeap::new(),
                min_heap: false,
            },
            _ => return None,
        })
    }

    pub fn len(&self) -> usize {
        match self {
            Object::Vector(e) | Object::Stack(e) | Object::List(e) => e.len(),
            Object::Array { n, .. } => *n,
            Object::Deque(q) | Object::Queue(q) => q.len(),
            Object::String(s) => s.len(),
            Object::Map(m) => m.len(),
            Object::UnorderedMap(m) => m.len(),
            Object::Set(s) => s.len(),
            Object::UnorderedSet(s) => s.len(),
            Object::PriorityQueue { heap, .. } => heap.len(),
            Object::Pair { .. } => 2,
            Object::Class { .. } | Object::Closure { .. } | Object::Functor { .. } => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        match self {
            Object::Vector(e) | Object::Stack(e) | Object::List(e) => e.clear(),
            Object::Array { elems, n } => {
                for e in elems.iter_mut().take(*n) {
                    *e = Value::Int(0);
                }
            }
            Object::Deque(q) | Object::Queue(q) => q.clear(),
            Object::String(s) => s.clear(),
            Object::Map(m) => m.clear(),
            Object::UnorderedMap(m) => m.clear(),
            Object::Set(s) => s.clear(),
            Object::UnorderedSet(s) => s.clear(),
            Object::PriorityQueue { heap, .. } => heap.clear(),
            Object::Pair { .. }
            | Object::Class { .. }
            | Object::Closure { .. }
            | Object::Functor { .. } => {}
        }
    }

    /// Snapshot for `Alloc`: `(size, elems, entries)`.
    /// Sequences use `elems`; maps/sets use `entries`.
    pub fn alloc_snapshot(&self) -> (usize, Vec<Value>, Vec<crate::event::AllocEntry>) {
        use crate::event::AllocEntry;
        match self {
            Object::Vector(e) | Object::Stack(e) | Object::List(e) => (e.len(), e.clone(), vec![]),
            Object::Array { elems, n } => (*n, elems.clone(), vec![]),
            Object::Deque(q) | Object::Queue(q) => (q.len(), q.iter().cloned().collect(), vec![]),
            Object::Pair { first, second } => (2, vec![first.clone(), second.clone()], vec![]),
            Object::String(s) => (s.len(), s.chars().map(Value::Char).collect(), vec![]),
            Object::Map(m) => (
                m.len(),
                vec![],
                m.iter()
                    .map(|(k, v)| AllocEntry {
                        key: k.clone(),
                        value: Some(v.clone()),
                    })
                    .collect(),
            ),
            Object::UnorderedMap(m) => (
                m.len(),
                vec![],
                m.iter()
                    .map(|(k, v)| AllocEntry {
                        key: k.clone(),
                        value: Some(v.clone()),
                    })
                    .collect(),
            ),
            Object::Set(s) => (
                s.len(),
                vec![],
                s.iter()
                    .map(|k| AllocEntry {
                        key: k.clone(),
                        value: None,
                    })
                    .collect(),
            ),
            Object::UnorderedSet(s) => (
                s.len(),
                vec![],
                s.iter()
                    .map(|k| AllocEntry {
                        key: k.clone(),
                        value: None,
                    })
                    .collect(),
            ),
            Object::PriorityQueue { heap, min_heap } => {
                let v: Vec<Value> = heap
                    .iter()
                    .copied()
                    .map(|x| Value::Int(if *min_heap { -x } else { x }))
                    .collect();
                (v.len(), v, vec![])
            }
            Object::Class { fields, .. } => {
                (fields.len(), fields.values().cloned().collect(), vec![])
            }
            Object::Closure { .. } | Object::Functor { .. } => (0, vec![], vec![]),
        }
    }
}

/// Heap object identity.
pub type ObjId = u64;

/// Address of a storage location (visualizer-friendly ADT, not raw bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum Address {
    Null,
    /// Slot in call stack frame `frame` (0 = oldest).
    Stack {
        frame: usize,
        name: String,
    },
    Heap(ObjId),
    Index {
        obj: ObjId,
        index: usize,
    },
    Field {
        obj: ObjId,
        field: String,
    },
    MapEntry {
        obj: ObjId,
        key: String,
    },
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Null => write!(f, "null"),
            Address::Stack { frame, name } => write!(f, "stack[{frame}].{name}"),
            Address::Heap(id) => write!(f, "heap#{id}"),
            Address::Index { obj, index } => write!(f, "heap#{obj}[{index}]"),
            Address::Field { obj, field } => write!(f, "heap#{obj}.{field}"),
            Address::MapEntry { obj, key } => write!(f, "heap#{obj}[{key}]"),
        }
    }
}

/// Runtime values (by-value primitives + heap handles + pointers/refs).
///
/// JSON: `{ "kind": "Int", "value": 42 }`.  
/// `Object`’s `value` is a **heap id**, not nested data — see `docs/events.md`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Void,
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(char),
    /// String payload for event keys / display (not a heap object).
    Str(String),
    Nullptr,
    /// Heap object id (`ObjId`).
    Object(ObjId),
    /// Reseating pointer.
    Ptr(Address),
    /// Non-reseating reference (alias).
    Ref(Address),
}

impl Value {
    pub fn as_bool(&self) -> Result<bool, String> {
        Ok(match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Char(c) => *c != '\0',
            Value::Str(s) => !s.is_empty(),
            Value::Nullptr => false,
            Value::Object(_) => true,
            Value::Ptr(Address::Null) => false,
            Value::Ptr(_) | Value::Ref(_) => true,
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

    pub fn as_address(&self) -> Option<&Address> {
        match self {
            Value::Ptr(a) | Value::Ref(a) => Some(a),
            Value::Nullptr => Some(&Address::Null),
            _ => None,
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
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Nullptr => write!(f, "nullptr"),
            Value::Object(id) => write!(f, "obj#{id}"),
            Value::Ptr(a) => write!(f, "ptr->{a}"),
            Value::Ref(a) => write!(f, "ref->{a}"),
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
