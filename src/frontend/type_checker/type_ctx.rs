use std::collections::HashMap;
use crate::frontend::type_checker::typ::Type::Error;
use crate::frontend::type_checker::typ::{Type, ExprId};
/// 类型变量,用于推导过程中"暂时未知,稍后统一"的类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

pub struct TypeCtxt {
    next_var_id: u32,
    substitutions: HashMap<TypeVar, Type>,   // 类型变量 -> 已解出的具体类型
    expr_types: HashMap<ExprId, Type>,       // 表达式节点 -> 最终类型标注(供代码生成阶段用)
}

impl TypeCtxt {
    pub fn new() -> Self{Self { next_var_id: 0, substitutions: HashMap::new(), expr_types: HashMap::new() }}
    pub fn fresh_type_var(&mut self) -> Type{
        Error
    }             // 生成新的 Type::Unknown(id)
    pub fn substitute(&mut self, var: TypeVar, ty: Type){}
    pub fn resolve_type(&self, ty: &Type) -> Type{Error}    // 递归替换所有已知的类型变量
    pub fn record_expr_type(&mut self, expr_id: ExprId, ty: Type){}
    pub fn get_expr_type(&self, expr_id: ExprId) -> Option<&Type>{None}
}