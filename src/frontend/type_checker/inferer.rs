use crate::frontend::ast::{ stmt::*, expr::*}; 
use crate::common::{DiagCtxt, span::Span};
use crate::frontend::sema_checker::{
    sema_ctx::SemaCtxt
};
use crate::frontend::type_checker::{
    typ::Type,
};
pub struct TypeInferer;

impl TypeInferer {
    pub fn new() -> Self {
        TypeInferer
    }
    // --- 表达式类型推导,返回推导出的类型 ---
    pub fn infer_expr(&mut self, expr: &Expr, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}

    fn infer_literal(&mut self, lit: &Literal) -> Type{Type::Error}
    fn infer_ident(&mut self, name: &str, span: Span, ctx: &SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_binary(&mut self, op: &BinaryOp, lhs: &Expr, rhs: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_unary(&mut self, op: &UnaryOp, operand: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_call(&mut self, callee: &Expr, args: &[Expr], span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_field_access(&mut self, base: &Expr, field: &str, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_index(&mut self, base: &Expr, index: &Expr, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_if_expr(&mut self, cond: &Expr, then: &Block, els: &Option<Block>, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_struct_literal(&mut self, name: &str, fields: &[(String, Expr)], span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    fn infer_array_literal(&mut self, elems: &[Expr], span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
    // fn infer_tuple(&mut self, elems: &[Expr], ctx：&mut SemaCtxt, diag：&mut DiagCtxt) -> Type{}

    // --- let 语句的双向推导:有标注时检查,无标注时从初始值推导 ---
    fn infer_let_binding(&mut self, declared_ty: &Option<Type>, init: &Option<Expr>, span: Span, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> Type{Type::Error}
}