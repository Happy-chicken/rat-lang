use crate::common::span::Span;
use crate::frontend::ast::printer::{AstPrint, branch, next_prefix};
use std::fmt::Write;
#[derive(Debug)]
pub struct ExprNode {
    pub span: Span,
    pub expr: Expr,
}

#[derive(Debug)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Float(f32),
    Char(char),
    StringLiteral(String),

    Variable(String),

    Assign {
        target: Box<ExprNode>,
        value: Box<ExprNode>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<ExprNode>,
        rhs: Box<ExprNode>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<ExprNode>,
    },
    Call {
        callee: Box<ExprNode>,
        args: Vec<ExprNode>,
    },
    Member {
        object: Box<ExprNode>,
        field: String,
    },
    Index {
        object: Box<ExprNode>,
        index: Box<ExprNode>,
    },
    List {
        elements: Vec<ExprNode>,
    },
    New {
        cons: String,
        args: Vec<ExprNode>,
    },
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,

    Eq,
    NotEq,

    Lt,
    Gt,

    And,
    Or,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
    Inc,
    Dec,
    Deref,
    AddrOf,
}

// ---- ExprNode ----
impl AstPrint for ExprNode {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        // 可选择忽略 span，只打印表达式
        self.expr.print(prefix, is_last, output)
    }
}

// ---- Expr ----
impl AstPrint for Expr {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        let branch_str = branch(is_last);
        match self {
            // 字面量 / 变量 —— writeln! 直接返回 Result
            Expr::Int(v) => writeln!(output, "{}{}Int({})", prefix, branch_str, v),
            Expr::Bool(v) => writeln!(output, "{}{}Bool({})", prefix, branch_str, v),
            Expr::Float(v) => writeln!(output, "{}{}Float({})", prefix, branch_str, v),
            Expr::Char(c) => writeln!(
                output,
                "{}{}Char('{}')",
                prefix,
                branch_str,
                escape_char(*c)
            ),
            Expr::StringLiteral(s) => writeln!(
                output,
                "{}{}String(\"{}\")",
                prefix,
                branch_str,
                escape_str(s)
            ),
            Expr::Variable(name) => writeln!(output, "{}{}Variable({})", prefix, branch_str, name),

            // Assign —— 块末尾须显式 Ok(())
            Expr::Assign { target, value } => {
                writeln!(output, "{}{}Assign", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}├── Target:", child)?;
                target.print(&format!("{}│   ", child), true, output)?;
                writeln!(output, "{}└── Value:", child)?;
                value.print(&format!("{}    ", child), true, output)?;
                Ok(())
            }
            // Binary —— 块末尾须显式 Ok(())
            Expr::Binary { op, lhs, rhs } => {
                writeln!(output, "{}{}Binary({:?})", prefix, branch_str, op)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}├── Lhs:", child)?;
                lhs.print(&format!("{}│   ", child), true, output)?;
                writeln!(output, "{}└── Rhs:", child)?;
                rhs.print(&format!("{}    ", child), true, output)?;
                Ok(())
            }
            // Unary —— 块末尾须显式 Ok(())
            Expr::Unary { op, expr } => {
                writeln!(output, "{}{}Unary({:?})", prefix, branch_str, op)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}└── Expr:", child)?;
                expr.print(&format!("{}    ", child), true, output)?;
                Ok(())
            }
            // Call —— 去掉分号，让 print_expr_list 的返回值作为块返回值
            Expr::Call { callee, args } => {
                writeln!(output, "{}{}Call", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);
                let has_args = !args.is_empty();

                // callee 不是最后一个兄弟（如果后面还有 args）
                writeln!(output, "{}├── callee:", child)?;
                callee.expr.print(
                    &format!("{}│   ", child),
                    true, // callee 作为第一个子节点，后面可能还有 args
                    output,
                )?;

                if has_args {
                    // args 是最后一个兄弟
                    writeln!(output, "{}└── args:", child)?;
                    let args_prefix = format!("{}    ", child); // “args:” 标签下的缩进前缀
                    for (i, arg) in args.iter().enumerate() {
                        arg.print(&args_prefix, i == args.len() - 1, output)?;
                    }
                }
                Ok(())
            }
            // Expr::Call { callee, args } => {
            //     callee.print(&format!("{}{}", prefix, branch_str), is_last, output)?;
            //     print_expr_list(args, prefix, is_last, output) // <-- 注意没有分号
            // }
            // Member —— 块末尾须显式 Ok(())
            Expr::Member { object, field } => {
                writeln!(output, "{}{}Member({})", prefix, branch_str, field)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}└── Object:", child)?;
                object.print(&format!("{}    ", child), true, output)?;
                Ok(())
            }
            // Index —— 块末尾须显式 Ok(())
            Expr::Index { object, index } => {
                writeln!(output, "{}{}Index", prefix, branch_str)?;
                let child = next_prefix(prefix, is_last);
                writeln!(output, "{}├── Object:", child)?;
                object.print(&format!("{}│   ", child), true, output)?;
                writeln!(output, "{}└── Index:", child)?;
                index.print(&format!("{}    ", child), true, output)?;
                Ok(())
            }
            // List —— 去掉分号
            Expr::List { elements } => {
                writeln!(output, "{}{}List", prefix, branch_str)?;
                print_expr_list(elements, prefix, is_last, output) // 无分号
            }
            // New —— 去掉分号
            Expr::New { cons, args } => {
                writeln!(output, "{}{}New({})", prefix, branch_str, cons)?;
                print_expr_list(args, prefix, is_last, output) // 无分号
            }
        }
    }
}

/// 辅助：打印 Vec<ExprNode> 形式的子节点列表。
fn print_expr_list(
    exprs: &[ExprNode],
    prefix: &str,
    is_last_parent: bool,
    output: &mut impl Write,
) -> std::fmt::Result {
    let child = next_prefix(prefix, is_last_parent);
    if exprs.is_empty() {
        writeln!(output, "{}└── <empty>", child)?;
    } else {
        for (i, expr) in exprs.iter().enumerate() {
            expr.print(&child, i == exprs.len() - 1, output)?;
        }
    }
    Ok(())
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        '\\' => "\\\\".into(),
        '\'' => "\\'".into(),
        _ => c.to_string(),
    }
}

fn escape_str(s: &str) -> String {
    s.escape_debug().to_string()
}
