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
    Array(usize, Box<Type>),

    Class(String),
}
