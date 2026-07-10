use std::rc::Rc;
use std::cell::RefCell;
use super::scope::{Scope, ScopeKind};
use super::symbol::{Symbol, SymbolKind};

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

    pub fn exit_scope(&mut self) -> Scope {
        assert!(self.scopes.len() >= 1, "cannot pop the global scope");
        self.scopes.pop().unwrap()
    }

    /// 在当前作用域声明符号；重复定义返回 Err
    pub fn declare(&mut self, mut symbol: Symbol) -> Result<(), Rc<RefCell<Symbol>>> {
        symbol.scope_depth = self.scopes.len() - 1;
        let current = self.scopes.last_mut().expect("at least global scope exists");
        match current.insert(symbol) {
            Some(existing) => Err(existing), // 冲突，返回旧符号供报错
            None => Ok(()),
        }
    }

    pub fn resolve_global(&self, name: &str) -> Option<Rc<RefCell<Symbol>>> {
        self.scopes.first().unwrap().get(name)
    }

    /// 从内到外逐层查找（词法作用域链）
    pub fn resolve(&self, name: &str) -> Option<Rc<RefCell<Symbol>>> {
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

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn dump(&self) {
        println!("=== Symbol Table ===");
        for (i, scope) in self.scopes.iter().enumerate() {
            let kind_name = match scope.kind {
                ScopeKind::Global => "Global",
                ScopeKind::Function => "Function",
                ScopeKind::Block => "Block",
                ScopeKind::Loop => "Loop",
            };
            println!("Scope[{}] {}:", i, kind_name);
            for (name, sym) in &scope.symbols {
                let sym = sym.borrow();
                let kind_str = match &sym.kind {
                    SymbolKind::Variable { is_mutable, is_initialized } => {
                        format!("variable (mut={}, init={})", is_mutable, is_initialized)
                    }
                    SymbolKind::Function { params, return_type } => {
                        format!("function({}) -> {:?}", params.len(), return_type)
                    }
                    SymbolKind::Type => "type".to_string(),
                    SymbolKind::Parameter { is_ref } => {
                        format!("parameter (ref={})", is_ref)
                    }
                    SymbolKind::Class { fields, .. } => {
                        format!("class ({} fields)", fields.len())
                    }
                    SymbolKind::Trait { .. } => "trait".to_string(),
                };
                println!("  {} : {} @ depth={}", name, kind_str, sym.scope_depth);
            }
        }
        println!("====================");
    }
}