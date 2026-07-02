#[derive(Debug, Clone)]
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
