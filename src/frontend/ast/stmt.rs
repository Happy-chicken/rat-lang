use crate::frontend::ast::expr::ExprNode;
use crate::frontend::ast::printer::{AstPrint, branch, next_prefix};
use crate::frontend::ast::typ::Type;
use std::fmt::Write;
use crate::common::span::Span;
#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<StmtNode>,
}

#[derive(Debug)]
pub struct StmtNode {
    pub span: Span,
    pub stmt: Stmt,
}

#[derive(Debug)]
pub enum Stmt {
    VarDef {
        name: String,
        ty: Option<Type>,
        init: Option<ExprNode>,
    },

    If {
        condition: ExprNode,
        then_branch: Block,
        elif_branch: Vec<(ExprNode, Block)>,
        else_branch: Block,
    },
    Loop {
        condition: ExprNode,
        body: Block,
    },

    ExprStmt(ExprNode),
    Return(Option<ExprNode>),
    Break,
    Continue,
    BlockStmt(Block),
}

// ---- Block ----
impl AstPrint for Block {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        writeln!(output, "{}└──Block", prefix)?;
        let child_prefix = next_prefix(prefix, is_last);
        let count = self.stmts.len();
        for (i, stmt_node) in self.stmts.iter().enumerate() {
            stmt_node.stmt.print(&child_prefix, i == count - 1, output)?;
        }
        Ok(())
    }
}

// ---- Stmt ----
impl AstPrint for Stmt {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        match self {
            Stmt::VarDef { name, ty, init } => {
                write!(output, "{}{}VarDef({}: {:?}", prefix, branch_str, name, ty)?;
                if let Some(init) = init {
                    writeln!(output, ") with init")?;
                    init.print(&next_prefix(prefix, is_last), true, output)?;
                } else {
                    writeln!(output, ")")?;
                }
            }
            Stmt::If {
                condition,
                then_branch,
                elif_branch,
                else_branch,
            } => {
                writeln!(output, "{}{}If", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);

                // condition
                writeln!(output, "{}├── Condition:", child)?;
                condition.print(&format!("{}│   ", child), true, output)?;

                // then branch
                writeln!(output, "{}├── Then:", child)?;
                let has_elif_or_else = !elif_branch.is_empty() || !else_branch.stmts.is_empty();
                then_branch.print(
                    &format!(
                        "{}{}",
                        child,
                        if has_elif_or_else { "│   " } else { "    " }
                    ),
                    true,
                    output,
                )?;

                // elif branches
                let elif_count = elif_branch.len();
                for (i, (cond, stmt)) in elif_branch.iter().enumerate() {
                    writeln!(output, "{}├── Elif:", child)?;
                    let is_last_elif = i == elif_count - 1 && else_branch.stmts.is_empty();
                    let next = format!("{}{}", child, if is_last_elif { "    " } else { "│   " });
                    cond.print(&format!("{}│   ", child), true, output)?;
                    // 把 Elif 里的 Stmt 当单个节点打印
                    stmt.print(&next, true, output)?;
                }

                // else branch
                if !else_branch.stmts.is_empty() {
                    writeln!(output, "{}└── Else:", child)?;
                    else_branch.print(&format!("{}    ", child), true, output)?;
                }
            }
            Stmt::Loop { condition, body } => {
                writeln!(output, "{}{}Loop", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}├── Condition:", child)?;
                condition.print(&format!("{}│   ", child), true, output)?;
                writeln!(output, "{}└── Body:", child)?;
                body.print(&format!("{}    ", child), true, output)?;
            }
            Stmt::ExprStmt(expr) => {
                writeln!(output, "{}{}ExprStmt: ", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);
                // 简单地在一行内显示简短表达式，也可递归；这里选择内联简化
                expr.print(&child, true, output)?;
                // writeln!(output, "{}", expr_to_string(expr))?;
            }
            Stmt::Return(expr) => {
                write!(output, "{}{}Return", prefix, branch_str)?;
                if let Some(e) = expr {
                    writeln!(output, ":")?;
                    e.print(&next_prefix(prefix, is_last), true, output)?;
                } else {
                    writeln!(output)?;
                }
            }
            Stmt::Break => {
                writeln!(output, "{}{}Break", prefix, branch_str)?;
            }
            Stmt::Continue => {
                writeln!(output, "{}{}Continue", prefix, branch_str)?;
            }
            Stmt::BlockStmt(block) => {
                writeln!(output, "{}{}BlockStmt", prefix, branch_str)?;
                block.print(&next_prefix(prefix, is_last), true, output)?;
            }
        }
        Ok(())
    }
}
