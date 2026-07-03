use crate::common::span::Span;
use crate::frontend::type_checker::typ::Type;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: String,
    pub ty: Option<Type>, 
    pub scope_depth: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Variable {
        is_mutable: bool,
        is_initialized: bool,
    },
    Function {
        params: Vec<Type>,
        return_type: Type,
    },
    Type,
    Parameter {
        is_ref: bool,
    },
    Class {
        fields: HashMap<String, Type>,
    },
    Trait,
}

impl Symbol {
    pub fn new( name: impl Into<String>, ty: Option<Type>, is_mutable: bool, span: Span) -> Self {
        let kind = SymbolKind::Variable {
            is_mutable,
            is_initialized: false,
        };
        Symbol { kind, name: name.into(), ty, scope_depth: 0, span }
    }
}