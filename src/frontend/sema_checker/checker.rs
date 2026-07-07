use crate::frontend::sema_checker::{pass::Pass, sema_ctx::SemaCtxt};
use crate::frontend::ast::{Program, expr::*, stmt::*, item::*};
use crate::common::{span::Span,DiagCtxt};

// pub struct ParamInfo {
//     pub name: String,
//     pub is_ref: bool,
//     pub span: Span,
// }
pub struct SemaChecker {}

impl SemaChecker {
    pub fn new() -> Self {
        Self {}
    }
}

impl Pass for SemaChecker {
    fn name(&self) -> &'static str {
        "sema_checker"
    }

    fn run(&mut self, program: & Program, ctx: &mut super::sema_ctx::SemaCtxt, diag: &mut DiagCtxt) -> bool {
        true
    }
}

impl SemaChecker {
    // --- 顶层 ---
    fn check_function(&mut self, params: &[Parameter], body: &Block, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}

    // --- 语句 ---
    fn check_stmt(&mut self, stmt: &Stmt, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_let_stmt(&mut self, name: &str, init: &Option<Expr>, is_mut: bool, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_block(&mut self, block: &Block, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_if_stmt(&mut self, cond: &Expr, then: &Block, els: &Option<Block>, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_while_stmt(&mut self, cond: &Expr, body: &Block, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_return_stmt(&mut self, expr: &Option<Expr>, span: Span, ctx: &SemaCtxt, diag: &mut DiagCtxt){}
    fn check_break_continue(&mut self, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_assign_stmt(&mut self, target: &Expr, value: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}

    // --- 表达式(只做符号解析,不做类型检查) ---
    fn check_expr(&mut self, expr: &Expr, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_ident_use(&mut self, name: &str, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_call_expr(&mut self, callee: &Expr, args: &[Expr], span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_field_access(&mut self, base: &Expr, field: &str, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}

    // --- 声明约束 ---
    fn check_mutability(&mut self, target: &Expr, ctx: &mut SemaCtxt, diag: &mut DiagCtxt){}
    fn check_duplicate_params(&mut self, params: &[Parameter], diag: &mut DiagCtxt){}

    // --- 收尾 ---
    // fn check_unused_variables(&mut self, popped_scope: &Scope, diag: &mut DiagCtxt){}
}