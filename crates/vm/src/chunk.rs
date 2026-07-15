use rscpp_ast::Span;
use rscpp_runtime::Value;

/// Stack-machine opcodes.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    LoadConst(u16),
    LoadLocal(u8),
    StoreLocal(u8),
    Pop,
    Dup,

    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    Not,

    CmpLt,
    CmpLe,
    CmpGt,
    CmpGe,
    CmpEq,
    CmpNe,

    Jump(u16),
    JumpIfFalse(u16),

    /// Call user function by program index; argc on stack above args.
    Call { func: u16, argc: u8 },
    /// `obj` then `argc` args on stack; method name in constants pool as StringLit-ish... use name idx.
    CallMethod { name: u16, argc: u8 },
    IndexGet,
    IndexSet,

    /// Push empty container by name constant ("vector", "map", …).
    NewEmpty { type_name: u16 },
    /// pair(a,b) — two values on stack.
    MakePair,

    Return,

    /// Emit Step event for this op's span.
    Step,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub ops: Vec<Op>,
    pub spans: Vec<Span>,
    pub constants: Vec<Value>,
    pub string_pool: Vec<String>,
    pub local_names: Vec<String>,
    pub arity: u8,
}

impl Chunk {
    pub fn new(name: impl Into<String>, arity: u8) -> Self {
        Self {
            name: name.into(),
            ops: Vec::new(),
            spans: Vec::new(),
            constants: Vec::new(),
            string_pool: Vec::new(),
            local_names: Vec::new(),
            arity,
        }
    }

    pub fn emit(&mut self, op: Op, span: Span) -> usize {
        let i = self.ops.len();
        self.ops.push(op);
        self.spans.push(span);
        i
    }

    pub fn add_const(&mut self, v: Value) -> u16 {
        let i = self.constants.len() as u16;
        self.constants.push(v);
        i
    }

    pub fn add_string(&mut self, s: impl Into<String>) -> u16 {
        let s = s.into();
        if let Some(i) = self.string_pool.iter().position(|x| x == &s) {
            return i as u16;
        }
        let i = self.string_pool.len() as u16;
        self.string_pool.push(s);
        i
    }

    pub fn patch_jump(&mut self, at: usize, target: u16) {
        match &mut self.ops[at] {
            Op::Jump(t) | Op::JumpIfFalse(t) => *t = target,
            other => panic!("patch_jump on non-jump {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Chunk>,
    pub main_index: Option<usize>,
}

impl Program {
    pub fn find(&self, name: &str) -> Option<usize> {
        self.functions.iter().position(|c| c.name == name)
    }
}
