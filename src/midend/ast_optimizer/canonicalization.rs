//! Expression canonicalization: normalize expression form for consistent matching.
//! Constants moved to RHS for commutative ops:  1 + x → x + 1,  0 == x → x == 0

use crate::frontend::ast::expr::{BinaryOp, Expr, ExprNode};
use crate::frontend::ast::item::FunctionDef;
use super::utils::{is_literal, walk_block_exprs};
use super::AstPass;

pub struct Canonicalization;

impl AstPass for Canonicalization {
    fn name(&self) -> &'static str { "Canonicalization" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        let mut changed = false;
        walk_block_exprs(&mut func.body, &mut |expr| {
            if Self::canonicalize(expr) { changed = true; }
        });
        changed
    }
}

/// Commutative operations where constants should be on the RHS.
const COMMUTATIVE: &[BinaryOp] = &[
    BinaryOp::Add, BinaryOp::Mul,
    BinaryOp::Eq, BinaryOp::NotEq,
    BinaryOp::And, BinaryOp::Or,
];

impl Canonicalization {
    fn canonicalize(expr: &mut ExprNode) -> bool {
        if let Expr::Binary { op, lhs, rhs } = &mut expr.expr {
            if COMMUTATIVE.contains(op) && is_literal(&lhs.expr) && !is_literal(&rhs.expr) {
                std::mem::swap(lhs, rhs);
                return true;
            }
        }
        false
    }
}
