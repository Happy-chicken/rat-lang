//! Dead branch elimination: fold constant conditions on if/while.
//! if true { A } else { B } → A
//! if false { A } else { B } → B
//! while false { ... } → removed

use std::mem;

use crate::frontend::ast::expr::{Expr, Literal};
use crate::frontend::ast::stmt::{Block, Stmt, StmtNode};
use crate::frontend::ast::item::FunctionDef;
use super::AstPass;

pub struct DeadBranchElimination;

impl AstPass for DeadBranchElimination {
    fn name(&self) -> &'static str { "DeadBranchElimination" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        Self::optimize_block(&mut func.body)
    }
}

impl DeadBranchElimination {
    fn optimize_block(block: &mut Block) -> bool {
        let mut changed = false;
        let mut i = 0;
        while i < block.stmts.len() {
            let removed = Self::optimize_stmt(&mut block.stmts[i]);
            if removed {
                changed = true;
                let stmt = &block.stmts[i];
                if matches!(&stmt.stmt, Stmt::Return(_) | Stmt::Break | Stmt::Continue) {
                    block.stmts.truncate(i + 1);
                    break;
                }
            }
            if let Stmt::BlockStmt(inner) = &mut block.stmts[i].stmt {
                if Self::optimize_block(inner) { changed = true; }
            }
            if let Stmt::If { then_branch, elif_branch, else_branch, .. } = &mut block.stmts[i].stmt {
                if Self::optimize_block(then_branch) { changed = true; }
                for (_, b) in elif_branch { if Self::optimize_block(b) { changed = true; } }
                if Self::optimize_block(else_branch) { changed = true; }
            }
            if let Stmt::Loop { body, .. } = &mut block.stmts[i].stmt {
                if Self::optimize_block(body) { changed = true; }
            }
            i += 1;
        }
        changed
    }

    fn optimize_stmt(stmt: &mut StmtNode) -> bool {
        match &mut stmt.stmt {
            Stmt::If { condition, then_branch, elif_branch, else_branch } => {
                if matches!(&condition.expr, Expr::Literal(Literal::Bool(true))) {
                    let taken = mem::replace(then_branch, Block { stmts: vec![] });
                    stmt.stmt = Stmt::BlockStmt(taken);
                    return true;
                }
                if matches!(&condition.expr, Expr::Literal(Literal::Bool(false))) {
                    for (cond, branch) in elif_branch.iter_mut() {
                        if matches!(&cond.expr, Expr::Literal(Literal::Bool(true))) {
                            let taken = mem::replace(branch, Block { stmts: vec![] });
                            stmt.stmt = Stmt::BlockStmt(taken);
                            return true;
                        }
                    }
                    let taken = mem::replace(else_branch, Block { stmts: vec![] });
                    stmt.stmt = Stmt::BlockStmt(taken);
                    return true;
                }
            }
            Stmt::Loop { condition, .. } => {
                if matches!(&condition.expr, Expr::Literal(Literal::Bool(false))) {
                    stmt.stmt = Stmt::BlockStmt(Block { stmts: vec![] });
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}
