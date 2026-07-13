//! Boolean simplification: truth tables + double negation.
//! x && true → x,  false && x → false,  !!x → x,  etc.

use crate::frontend::ast::expr::{BinaryOp, Expr, ExprNode, UnaryOp};
use crate::frontend::ast::item::FunctionDef;
use super::utils::{is_bool, is_literal, make_bool, walk_block_exprs};
use super::AstPass;

pub struct BooleanSimplifier;

impl AstPass for BooleanSimplifier {
    fn name(&self) -> &'static str { "BooleanSimplifier" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        let mut changed = false;
        walk_block_exprs(&mut func.body, &mut |expr| {
            if Self::simplify(expr) { changed = true; }
        });
        changed
    }
}

impl BooleanSimplifier {
    fn simplify(expr: &mut ExprNode) -> bool {
        match &expr.expr {
            Expr::Binary { op, lhs, rhs } => {
                let lhs_lit = is_literal(&lhs.expr);
                let rhs_lit = is_literal(&rhs.expr);

                match op {
                    BinaryOp::And => {
                        if rhs_lit && is_bool(&rhs.expr, true) {
                            *expr = (**lhs).clone(); return true;
                        }
                        if lhs_lit && is_bool(&lhs.expr, true) {
                            *expr = (**rhs).clone(); return true;
                        }
                        if rhs_lit && is_bool(&rhs.expr, false) || lhs_lit && is_bool(&lhs.expr, false) {
                            *expr = make_bool(false); return true;
                        }
                    }
                    BinaryOp::Or => {
                        if rhs_lit && is_bool(&rhs.expr, false) {
                            *expr = (**lhs).clone(); return true;
                        }
                        if lhs_lit && is_bool(&lhs.expr, false) {
                            *expr = (**rhs).clone(); return true;
                        }
                        if rhs_lit && is_bool(&rhs.expr, true) || lhs_lit && is_bool(&lhs.expr, true) {
                            *expr = make_bool(true); return true;
                        }
                    }
                    _ => {}
                }
            }
            Expr::Unary { op, expr: inner } => match op {
                UnaryOp::Not => {
                    if let Expr::Unary { op: UnaryOp::Not, expr: inner2 } = &inner.expr {
                        *expr = (**inner2).clone(); return true;
                    }
                }
                UnaryOp::Neg => {
                    if let Expr::Unary { op: UnaryOp::Neg, expr: inner2 } = &inner.expr {
                        *expr = (**inner2).clone(); return true;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        false
    }
}
