//! Constant folding: evaluate constant expressions at compile time.

use crate::frontend::ast::expr::{BinaryOp, Expr, ExprNode, Literal, UnaryOp};
use crate::frontend::ast::item::FunctionDef;
use super::utils::{walk_block_exprs, make_bool};
use super::AstPass;

pub struct ConstantFolder;

impl AstPass for ConstantFolder {
    fn name(&self) -> &'static str { "ConstantFolder" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        let mut changed = false;
        walk_block_exprs(&mut func.body, &mut |expr| {
            if Self::fold(expr) { changed = true; }
        });
        changed
    }
}

impl ConstantFolder {
    fn fold(expr: &mut ExprNode) -> bool {
        match &expr.expr {
            Expr::Binary { op, lhs, rhs } => {
                if let (Expr::Literal(l), Expr::Literal(r)) = (&lhs.expr, &rhs.expr) {
                    if let Some(result) = eval_binary(l, op, r) {
                        expr.expr = Expr::Literal(result);
                        return true;
                    }
                }
            }
            Expr::Unary { op, expr: inner } => {
                if let Expr::Literal(lit) = &inner.expr {
                    if let Some(result) = eval_unary(op, lit) {
                        expr.expr = Expr::Literal(result);
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}

fn eval_binary(lhs: &Literal, op: &BinaryOp, rhs: &Literal) -> Option<Literal> {
    match (lhs, rhs) {
        (Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(match op {
            BinaryOp::Add => a + b,
            BinaryOp::Sub => a - b,
            BinaryOp::Mul => a * b,
            BinaryOp::Div => if *b != 0 { a / b } else { return None; },
            BinaryOp::Eq => return Some(Literal::Bool(a == b)),
            BinaryOp::NotEq => return Some(Literal::Bool(a != b)),
            BinaryOp::Lt => return Some(Literal::Bool(a < b)),
            BinaryOp::Gt => return Some(Literal::Bool(a > b)),
            BinaryOp::Le => return Some(Literal::Bool(a <= b)),
            BinaryOp::Ge => return Some(Literal::Bool(a >= b)),
            BinaryOp::And | BinaryOp::Or => return None,
        })),
        (Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(match op {
            BinaryOp::Add => a + b,
            BinaryOp::Sub => a - b,
            BinaryOp::Mul => a * b,
            BinaryOp::Div => if *b != 0.0 { a / b } else { return None; },
            BinaryOp::Eq => return Some(Literal::Bool(a == b)),
            BinaryOp::NotEq => return Some(Literal::Bool(a != b)),
            BinaryOp::Lt => return Some(Literal::Bool(a < b)),
            BinaryOp::Gt => return Some(Literal::Bool(a > b)),
            BinaryOp::Le => return Some(Literal::Bool(a <= b)),
            BinaryOp::Ge => return Some(Literal::Bool(a >= b)),
            _ => return None,
        })),
        (Literal::Bool(a), Literal::Bool(b)) => match op {
            BinaryOp::And => Some(Literal::Bool(*a && *b)),
            BinaryOp::Or => Some(Literal::Bool(*a || *b)),
            BinaryOp::Eq => Some(Literal::Bool(a == b)),
            BinaryOp::NotEq => Some(Literal::Bool(a != b)),
            _ => None,
        },
        (Literal::Char(a), Literal::Char(b)) => match op {
            BinaryOp::Eq => Some(Literal::Bool(a == b)),
            BinaryOp::NotEq => Some(Literal::Bool(a != b)),
            BinaryOp::Lt => Some(Literal::Bool(a < b)),
            BinaryOp::Gt => Some(Literal::Bool(a > b)),
            _ => None,
        },
        _ => None,
    }
}

fn eval_unary(op: &UnaryOp, lit: &Literal) -> Option<Literal> {
    match op {
        UnaryOp::Neg => match lit {
            Literal::Int(i) => Some(Literal::Int(-i)),
            Literal::Float(f) => Some(Literal::Float(-f)),
            _ => None,
        },
        UnaryOp::Not => match lit {
            Literal::Bool(b) => Some(Literal::Bool(!b)),
            _ => None,
        },
        _ => None,
    }
}
