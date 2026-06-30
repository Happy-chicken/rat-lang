#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,

    Void,
    List(Box<Type>),

    Class(String),
}
