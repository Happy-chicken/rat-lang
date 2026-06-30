use crate::frontend::ast::expr::ExprNode;
use crate::frontend::ast::typ::Type;

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    VarDef {
        name: String,
        ty: Option<Type>,
        init: Option<ExprNode>,
    },
    Assign {
        target: ExprNode,
        value: ExprNode,
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
