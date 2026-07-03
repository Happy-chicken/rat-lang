use std::collections::HashMap;
use std::rc::Rc;
use super::symbol::Symbol;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Loop,      // 用于 break/continue 
}

pub struct Scope {
    pub kind: ScopeKind,
    pub symbols: HashMap<String, Rc<Symbol>>,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self { kind, symbols: HashMap::new() }
    }

    pub fn insert(&mut self, symbol: Rc<Symbol>) -> Option<Rc<Symbol>> {
        // 返回 Some 表示同一作用域内重复定义
        self.symbols.insert(symbol.name.clone(), symbol)
    }

    pub fn get(&self, name: &str) -> Option<Rc<Symbol>> {
        self.symbols.get(name).cloned()
    }
}