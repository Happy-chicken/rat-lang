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

pub type ExprId = u32;

impl Type {
    pub fn is_numeric(&self) -> bool{false}
    pub fn is_integer(&self) -> bool{false}
    pub fn is_float(&self) -> bool{false}
    pub fn is_unit(&self) -> bool{false}

    /// 相等性检查,Error 类型与任何类型都视为"相等"(阻止级联报错)
    pub fn compatible_with(&self, other: &Type) -> bool{false}

    /// 数值类型间隐式转换规则,如 i32 -> f64 是否允许
    pub fn can_coerce_to(&self, target: &Type) -> bool{false}

    pub fn display_name(&self) -> String{String::from("value")} // 用于诊断信息
}