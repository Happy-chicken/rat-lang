#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Ptr(Box<Type>),

    Void,
    List(Box<Type>),

    Class(String),
}
