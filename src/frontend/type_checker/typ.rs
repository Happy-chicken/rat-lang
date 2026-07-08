#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Prim(PrimType),
    Var(u32),
    List(Box<Type>),
    Ptr(Box<Type>),
    Func(Vec<Type>, Box<Type>),
    Class(String),
    TraitObject(String),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimType {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Void,
}

pub type ExprId = u32;

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Prim(PrimType::Int) | Type::Prim(PrimType::Float)
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Prim(PrimType::Int))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Prim(PrimType::Bool))
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Type::Prim(PrimType::Void))
    }

    pub fn display_name(&self) -> String {
        match self {
            Type::Prim(PrimType::Int) => "int".into(),
            Type::Prim(PrimType::Float) => "float".into(),
            Type::Prim(PrimType::Bool) => "bool".into(),
            Type::Prim(PrimType::Char) => "char".into(),
            Type::Prim(PrimType::Str) => "str".into(),
            Type::Prim(PrimType::Void) => "none".into(),
            Type::Var(id) => format!("?{}", id),
            Type::List(inner) => format!("list<{}>", inner.display_name()),
            Type::Ptr(inner) => format!("ptr<{}>", inner.display_name()),
            Type::Func(params, ret) => {
                let p: Vec<String> = params.iter().map(|t| t.display_name()).collect();
                format!("({}) -> {}", p.join(", "), ret.display_name())
            }
            Type::Class(name) => name.clone(),
            Type::TraitObject(name) => format!("dyn {}", name),
            Type::Error => "{error}".into(),
        }
    }
}
