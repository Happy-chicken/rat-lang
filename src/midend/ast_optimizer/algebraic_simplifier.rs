//! Algebraic simplification: identity/zero/annihilator rules.
//! x + 0 → x,  x * 1 → x,  x * 0 → 0,  x - x → 0,  x / 1 → x

use crate::frontend::ast::expr::{BinaryOp, Expr, ExprNode};
use crate::frontend::ast::item::FunctionDef;
use super::utils::{is_literal, is_int_or_float_zero, is_int_or_float_one, exprs_equal, make_int, make_float, walk_block_exprs};
use super::AstPass;

pub struct AlgebraicSimplifier;

impl AstPass for AlgebraicSimplifier {
    fn name(&self) -> &'static str { "AlgebraicSimplifier" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        let mut changed = false;
        walk_block_exprs(&mut func.body, &mut |expr| {
            if Self::simplify(expr) { changed = true; }
        });
        changed
    }
}

impl AlgebraicSimplifier {
    fn simplify(expr: &mut ExprNode) -> bool {
        if let Expr::Binary { op, lhs, rhs } = &expr.expr {
            let lhs_lit = is_literal(&lhs.expr);
            let rhs_lit = is_literal(&rhs.expr);

            match op {
                BinaryOp::Add => {
                    if rhs_lit && is_int_or_float_zero(&rhs.expr) {
                        *expr = (**lhs).clone(); return true;
                    }
                    if lhs_lit && is_int_or_float_zero(&lhs.expr) {
                        *expr = (**rhs).clone(); return true;
                    }
                }
                BinaryOp::Sub => {
                    if rhs_lit && is_int_or_float_zero(&rhs.expr) {
                        *expr = (**lhs).clone(); return true;
                    }
                    if exprs_equal(lhs, rhs) {
                        *expr = make_int(0); return true;
                    }
                }
                BinaryOp::Mul => {
                    if rhs_lit && is_int_or_float_one(&rhs.expr) {
                        *expr = (**lhs).clone(); return true;
                    }
                    if lhs_lit && is_int_or_float_one(&lhs.expr) {
                        *expr = (**rhs).clone(); return true;
                    }
                    if rhs_lit && is_int_or_float_zero(&rhs.expr) {
                        *expr = (**rhs).clone(); return true;
                    }
                    if lhs_lit && is_int_or_float_zero(&lhs.expr) {
                        *expr = (**lhs).clone(); return true;
                    }
                }
                BinaryOp::Div => {
                    if rhs_lit && is_int_or_float_one(&rhs.expr) {
                        *expr = (**lhs).clone(); return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}
