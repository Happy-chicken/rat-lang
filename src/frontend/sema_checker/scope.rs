use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
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
    pub symbols: HashMap<String, Rc<RefCell<Symbol>>>,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self { kind, symbols: HashMap::new() }
    }

    pub fn insert(&mut self, symbol: Symbol) -> Option<Rc<RefCell<Symbol>>> {
        // 返回 Some 表示同一作用域内重复定义
        let name = symbol.name.clone();
        let existing = self.symbols.get(&name).cloned();
        self.symbols.insert(name, Rc::new(RefCell::new(symbol)));
        existing
    }

    pub fn get(&self, name: &str) -> Option<Rc<RefCell<Symbol>>> {
        self.symbols.get(name).cloned()
    }
}