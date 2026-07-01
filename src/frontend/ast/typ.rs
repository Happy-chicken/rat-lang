#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    Str,

    Void,
    List(Box<Type>),

    Class(String),
}
