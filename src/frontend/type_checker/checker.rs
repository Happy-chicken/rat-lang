use crate::frontend::type_checker::{
    typ::Type, 
    inferer::TypeInferer};
use crate::frontend::ast::{Program, item::*, stmt::*, expr::*}; 
use crate::common::{DiagCtxt, span::Span};
use crate::frontend::sema_checker::{
    pass::Pass, 
    sema_ctx::SemaCtxt
};
pub struct TypeChecker {
    inferer: TypeInferer,
}

impl Pass for TypeChecker {
    fn name(&self) -> &'static str { "type_checker" }

    fn run(&mut self, program: &Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        true
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            inferer: TypeInferer::new(),
        }
    }

    fn check_function(&mut self, func: &FunctionDef, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}

    // --- 语句级别检查 ---
    fn check_stmt(&mut self, stmt: &Stmt, expected_return: &Type, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    // fn check_let_stmt(&mut self, ...){}
    fn check_assign_stmt(&mut self, target: &Expr, value: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_return_stmt(&mut self, expr: &Option<Expr>, expected: &Type, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}

    // --- 特定构造的类型规则 ---
    fn check_condition_is_bool(&mut self, cond: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_if_branches_match(&mut self, then_ty: &Type, else_ty: &Type, span: Span, diag: &mut DiagCtxt){}
    fn check_array_index_is_integer(&mut self, index_ty: &Type, span: Span, diag: &mut DiagCtxt){}
    fn check_call_arity_and_types(&mut self, params: &[Type], args: &[Type], span: Span, diag: &mut DiagCtxt){}
    fn check_binary_op_types(&mut self, op: &BinaryOp, lhs: &Type, rhs: &Type, span: Span, diag: &mut DiagCtxt) -> Type{
        Type::Error
    }
    fn check_struct_field_types(&mut self, struct_name: &str, given_fields: &[(String, Type)], span: Span, ctx: &SemaCtxt, diag: &mut DiagCtxt){}

    // --- 收尾:把 ctx.types 里所有 Unknown 变量强制求解,解不出的报错 ---
    fn finalize_inference(&mut self, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
}