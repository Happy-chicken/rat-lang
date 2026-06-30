use crate::common::span::Span;

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
        name: String,
        args: Vec<ExprNode>,
    },
    MemberAccess {
        object: Box<ExprNode>,
        field: String,
    },
    MethodCall {
        object: Box<ExprNode>,
        method: String,
        args: Vec<ExprNode>,
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
}
