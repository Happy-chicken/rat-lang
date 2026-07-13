//! Block simplification: flatten nested blocks, remove empty blocks.

use crate::frontend::ast::stmt::{Block, Stmt};
use crate::frontend::ast::item::FunctionDef;
use super::AstPass;

pub struct BlockSimplifier;

impl AstPass for BlockSimplifier {
    fn name(&self) -> &'static str { "BlockSimplifier" }

    fn run_on_function(&self, func: &mut FunctionDef) -> bool {
        let mut changed = false;
        flush_block(&mut func.body, &mut changed);
        changed
    }
}

fn flush_block(block: &mut Block, changed: &mut bool) {
    let mut i = 0;
    while i < block.stmts.len() {
        // Recursively simplify inner blocks first
        match &mut block.stmts[i].stmt {
            Stmt::BlockStmt(inner) => flush_block(inner, changed),
            Stmt::If { then_branch, elif_branch, else_branch, .. } => {
                flush_block(then_branch, changed);
                for (_, b) in elif_branch { flush_block(b, changed); }
                flush_block(else_branch, changed);
            }
            Stmt::Loop { body, .. } => flush_block(body, changed),
            _ => {}
        }

        // Flatten: replace BlockStmt with its contents
        if let Stmt::BlockStmt(inner) = &mut block.stmts[i].stmt {
            let mut inner_stmts = std::mem::replace(&mut inner.stmts, vec![]);
            if inner_stmts.is_empty() {
                // Remove empty block
                block.stmts.remove(i);
                *changed = true;
                continue;
            }
            // Inline the inner block's statements
            if inner_stmts.len() == 1 {
                block.stmts[i] = inner_stmts.remove(0);
            } else {
                block.stmts.splice(i..=i, inner_stmts);
            }
            *changed = true;
            continue;
        }

        i += 1;
    }
}
