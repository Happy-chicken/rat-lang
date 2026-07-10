use crate::common::span::Span;
use crate::frontend::ast::typ::Type;

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
        fields: Vec<(String, Type)>,
    },
    Trait {
        methods: Vec<String>,
    },
}

impl Symbol {
    fn new( name: impl Into<String>, ty: Option<Type>, is_mutable: bool, span: Span) -> Self {
        let kind = SymbolKind::Variable {
            is_mutable,
            is_initialized: false,
        };
        Symbol { kind, name: name.into(), ty, scope_depth: 0, span }
    }

    pub fn new_function(name: impl Into<String>, params: Vec<Type>, return_type: Type, span: Span) -> Self {
        let kind = SymbolKind::Function { params, return_type };
        Symbol { kind, name: name.into(), ty: None, scope_depth: 0, span }
    }

    pub fn new_parameter(name: impl Into<String>, ty: Type, is_ref: bool, span: Span) -> Self {
        let kind = SymbolKind::Parameter { is_ref };
        Symbol { kind, name: name.into(), ty: Some(ty), scope_depth: 0, span }
    }

    pub fn new_type(name: impl Into<String>, span: Span) -> Self {
        let kind = SymbolKind::Type;
        Symbol { kind, name: name.into(), ty: None, scope_depth: 0, span }
    }

    pub fn new_class(name: impl Into<String>, fields: Vec<(String, Type)>, span: Span) -> Self {
        let kind = SymbolKind::Class { fields };
        Symbol { kind, name: name.into(), ty: None, scope_depth: 0, span }
    }

    pub fn new_trait(name: impl Into<String>, methods: Vec<String>, span: Span) -> Self {
        let kind = SymbolKind::Trait { methods };
        Symbol { kind, name: name.into(), ty: None, scope_depth: 0, span }
    }

    pub fn new_variable(name: impl Into<String>, ty: Option<Type>, is_mutable: bool, span: Span) -> Self {
        let kind = SymbolKind::Variable {
            is_mutable,
            is_initialized: false,
        };
        Symbol { kind, name: name.into(), ty, scope_depth: 0, span }
    }

    pub fn is_type(&self) -> bool {
        matches!(self.kind, SymbolKind::Type)
    }

    pub fn is_callable(&self) -> bool {
        matches!(self.kind, SymbolKind::Function { .. } | SymbolKind::Class { .. })
    }

    pub fn kind_name(&self) -> &'static str {
        match &self.kind {
            SymbolKind::Variable { .. } => "variable",
            SymbolKind::Function { .. } => "function",
            SymbolKind::Type => "type",
            SymbolKind::Parameter { .. } => "parameter",
            SymbolKind::Class { .. } => "class",
            SymbolKind::Trait { .. } => "trait",
        }
    }


}