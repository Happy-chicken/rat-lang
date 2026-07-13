//! Shared utility functions for AST optimization passes.

use crate::common::span::Span;
use crate::frontend::ast::expr::{Expr, ExprNode, Literal};

pub fn is_int_or_float_zero(expr: &Expr) -> bool {
    matches!(expr, Expr::Literal(Literal::Int(0)) | Expr::Literal(Literal::Float(0.0)))
}

pub fn is_int_or_float_one(expr: &Expr) -> bool {
    matches!(expr, Expr::Literal(Literal::Int(1)) | Expr::Literal(Literal::Float(1.0)))
}

pub fn is_int_zero(expr: &Expr) -> bool {
    matches!(expr, Expr::Literal(Literal::Int(0)))
}

pub fn is_bool(expr: &Expr, val: bool) -> bool {
    matches!(expr, Expr::Literal(Literal::Bool(b)) if *b == val)
}

pub fn is_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Literal(_))
}

pub fn exprs_equal(a: &ExprNode, b: &ExprNode) -> bool {
    match (&a.expr, &b.expr) {
        (Expr::Variable(va), Expr::Variable(vb)) => va == vb,
        _ => false,
    }
}

pub fn make_int(i: i64) -> ExprNode {
    ExprNode {
        span: Span::new(0.into(), 0.into()),
        expr: Expr::Literal(Literal::Int(i)),
    }
}

pub fn make_float(f: f32) -> ExprNode {
    ExprNode {
        span: Span::new(0.into(), 0.into()),
        expr: Expr::Literal(Literal::Float(f)),
    }
}

pub fn make_bool(b: bool) -> ExprNode {
    ExprNode {
        span: Span::new(0.into(), 0.into()),
        expr: Expr::Literal(Literal::Bool(b)),
    }
}

/// Recursively optimizes expressions within a statement.
pub fn optimize_stmt_exprs<F>(stmt: &mut crate::frontend::ast::stmt::Stmt, f: &mut F)
where
    F: FnMut(&mut ExprNode),
{
    use crate::frontend::ast::stmt::Stmt;

    match stmt {
        Stmt::VarDef { init, .. } => {
            if let Some(e) = init { f(e); }
        }
        Stmt::Return(Some(e)) => f(e),
        Stmt::Return(None) => {}
        Stmt::ExprStmt(e) => f(e),
        Stmt::Break | Stmt::Continue => {}
        Stmt::BlockStmt(_) | Stmt::If { .. } | Stmt::Loop { .. } => {}
    }
}

/// Recursively walks an expression tree, calling `f` on each ExprNode.
/// Use for bottom-up transformations: call `f` after recursing.
pub fn walk_expr_post(expr: &mut ExprNode, f: &mut impl FnMut(&mut ExprNode)) {
    match &mut expr.expr {
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr_post(lhs, f);
            walk_expr_post(rhs, f);
        }
        Expr::Unary { expr: inner, .. } => {
            walk_expr_post(inner, f);
        }
        Expr::Call { callee, args } => {
            walk_expr_post(callee, f);
            for arg in args { walk_expr_post(arg, f); }
        }
        Expr::Assign { target, value } => {
            walk_expr_post(target, f);
            walk_expr_post(value, f);
        }
        Expr::Member { object, .. } => walk_expr_post(object, f),
        Expr::Index { object, index } => {
            walk_expr_post(object, f);
            walk_expr_post(index, f);
        }
        Expr::List { elements } => {
            for e in elements { walk_expr_post(e, f); }
        }
        Expr::Literal(_) | Expr::Variable(_) => {}
    }
    f(expr);
}

/// Recursively walks a block, calling `walk_expr_post` on expression nodes.
pub fn walk_block_exprs(block: &mut crate::frontend::ast::stmt::Block, f: &mut impl FnMut(&mut ExprNode)) {
    use crate::frontend::ast::stmt::Stmt;

    for stmt in &mut block.stmts {
        optimize_stmt_exprs(&mut stmt.stmt, f);
        match &mut stmt.stmt {
            Stmt::If { condition, then_branch, elif_branch, else_branch } => {
                walk_expr_post(condition, f);
                walk_block_exprs(then_branch, f);
                for (cond, branch) in elif_branch { walk_expr_post(cond, f); walk_block_exprs(branch, f); }
                walk_block_exprs(else_branch, f);
            }
            Stmt::Loop { condition, body } => {
                walk_expr_post(condition, f);
                walk_block_exprs(body, f);
            }
            Stmt::BlockStmt(block) => walk_block_exprs(block, f),
            _ => {}
        }
    }
}
