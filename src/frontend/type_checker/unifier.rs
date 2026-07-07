use crate::frontend::type_checker::{typ::Type, type_ctx::TypeCtxt, type_ctx::TypeVar};
use crate::common::error::{UnifyError};
pub struct Unifier<'a> {
    ctx: &'a mut TypeCtxt,
}

impl<'a> Unifier<'a> {
    pub fn new(ctx: &'a mut TypeCtxt) -> Self{
        Self { ctx }
    }

    /// 尝试让两个类型相等,可能绑定类型变量失败返回 Err 供上层报错
    pub fn unify(&mut self, a: &Type, b: &Type) -> Result<Type, UnifyError>{
        Ok(Type::Error)
    }

    fn unify_var(&mut self, var: TypeVar, ty: &Type) -> Result<Type, UnifyError>{
        Ok(Type::Error)
    }
    fn occurs_check(&self, var: TypeVar, ty: &Type) -> bool{false}  // 防止 T = Vec<T> 死循环
}

