use std::rc::Rc;
use super::scope::{Scope, ScopeKind};
use super::symbol::Symbol;

pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { scopes: vec![Scope::new(ScopeKind::Global)] }
    }

    pub fn enter_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope::new(kind));
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
        assert!(self.scopes.len() >= 1);
    }

    /// 在当前作用域声明符号；重复定义返回 Err
    pub fn declare(&mut self, symbol: Symbol) -> Result<(), Symbol> {
        let mut symbol = symbol;
        symbol.scope_depth = self.scopes.len() - 1;
        let rc = Rc::new(symbol);
        let current = self.scopes.last_mut().unwrap();
        match current.insert(rc.clone()) {
            Some(existing) => Err((*existing).clone()), // 冲突，返回旧符号供报错
            None => Ok(()),
        }
    }

    /// 从内到外逐层查找（词法作用域链）
    pub fn resolve(&self, name: &str) -> Option<Rc<Symbol>> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    /// 查找最近的某类作用域（用于 break/continue/return 检查）
    pub fn nearest_of_kind(&self, kind: ScopeKind) -> bool {
        self.scopes.iter().rev().any(|s| s.kind == kind)
    }

    pub fn current_scope_kind(&self) -> ScopeKind {
        self.scopes.last().unwrap().kind
    }
}