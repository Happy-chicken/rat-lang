#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Prim(PrimType),
    Array(usize, Box<Type>),       // 已知长度 + 元素类型
    List(Box<Type>),                // 元素类型
    Ptr(Box<Type>),
    Func(Vec<Type>, Option<Box<Type>>),    // 参数类型 → 返回类型
    Class(String),                 // 用户类
    TraitObject(String),           // dyn Trait
    Error,                         // 错误类型（用于错误恢复）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimType {
    Int, Float, Bool, Char, Void,
}
