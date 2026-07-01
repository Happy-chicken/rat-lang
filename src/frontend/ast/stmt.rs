use crate::frontend::ast::expr::ExprNode;
use crate::frontend::ast::printer::{AstPrint, next_prefix, branch};
use crate::frontend::ast::typ::Type;
use std::fmt::Write;
#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    VarDef {
        name: String,
        ty: Type,
        init: Option<ExprNode>,
    },

    If {
        condition: ExprNode,
        then_brach: Block,
        elif_brach: Vec<(ExprNode, Stmt)>,
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
}

// ---- Block ----
impl AstPrint for crate::frontend::ast::stmt::Block {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        writeln!(output, "{}└──Block", prefix)?;
        let child_prefix = next_prefix(prefix, is_last);
        let count = self.stmts.len();
        for (i, stmt) in self.stmts.iter().enumerate() {
            stmt.print(&child_prefix, i == count - 1, output)?;
        }
        Ok(())
    }
}

// ---- Stmt ----
impl AstPrint for crate::frontend::ast::stmt::Stmt {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        match self {
            Stmt::VarDef { name, ty, init } => {
                write!(output, "{}{}VarDef({}: {:?}", prefix, branch_str, name, ty)?;
                if let Some(init) = init {
                    writeln!(output, ") =")?;
                    init.print(&next_prefix(prefix, is_last), true, output)?;
                } else {
                    writeln!(output, ")")?;
                }
            }
            Stmt::If {
                condition,
                then_brach,
                elif_brach,
                else_branch,
            } => {
                writeln!(output, "{}{}If", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);

                // condition
                writeln!(output, "{}├── Condition:", child)?;
                condition.print(&format!("{}│   ", child), true, output)?;

                // then branch
                writeln!(output, "{}├── Then:", child)?;
                let has_elif_or_else = !elif_brach.is_empty() || !else_branch.stmts.is_empty();
                then_brach.print(&format!("{}{}", child, if has_elif_or_else { "│   " } else { "    " }), true, output)?;

                // elif branches
                let elif_count = elif_brach.len();
                for (i, (cond, stmt)) in elif_brach.iter().enumerate() {
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
                write!(output, "{}{}ExprStmt: ", prefix, branch_str)?;
                // 简单地在一行内显示简短表达式，也可递归；这里选择内联简化
                writeln!(output, "{}", expr_to_string(expr))?;
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
        }
        Ok(())
    }
}

/// 简单的表达式内联字符串（用于 ExprStmt 快速显示），也可直接复用 print。
fn expr_to_string(expr: &ExprNode) -> String {
    // 为了简单，直接调用 print 到一个 String，但这里避免深度递归太长；
    // 你可以改用更紧凑的格式，此处仅示例。
    let mut s = String::new();
    // 忽略写入错误
    let _ = expr.print("", true, &mut s);
    s.trim().to_string()
}