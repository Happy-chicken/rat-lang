use crate::frontend::ast::stmt::Block;
use crate::frontend::ast::typ::Type;

#[derive(Debug)]
pub enum Item {
    Function(Function),

    Class(Class),
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}
