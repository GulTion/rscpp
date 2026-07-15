use crate::ty::Ty;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Ty,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Func,
    Class,
    Field,
}

#[derive(Debug, Default)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn insert(&mut self, sym: Symbol) -> Option<Symbol> {
        self.symbols.insert(sym.name.clone(), sym)
    }

    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, sym: Symbol) -> Result<(), Symbol> {
        let scope = self.scopes.last_mut().unwrap();
        if let Some(prev) = scope.get(&sym.name) {
            // Allow redefinition of class after we only registered stub? For now reject same scope.
            return Err(prev.clone());
        }
        scope.insert(sym);
        Ok(())
    }

    pub fn redefine(&mut self, sym: Symbol) {
        self.scopes.last_mut().unwrap().insert(sym);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(s) = scope.get(name) {
                return Some(s);
            }
        }
        None
    }
}
