mod common;
mod frontend;
use common::DiagCtxt;
use common::location::SourceFile;
use common::span::Span;
use frontend::ast::{
    Program,
    expr::{Expr, ExprNode},
    item::{Function, Item},
    stmt::{Block, Stmt},
};
use frontend::lexer::Lexer;
fn main() {
    let src = r#" 
    def main() { var x = 42;
            var y = 3.14;
            var z = "Hello, world!";
            return x;
        }
    "#;
    let file = SourceFile::new("main.rs".to_string(), src.to_string());
    let mut diag_ctxt = DiagCtxt::new();
    diag_ctxt.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    for token in lexer {
        println!("{:?}", token);
        if token.kind == frontend::lexer::token::TokenKind::Error {
            let diag = diag_ctxt
                .struct_span_err(token.span, "unexpected character")
                .note("this token is invalid")
                .build();
            diag_ctxt.emit(diag);
        }
    }

    diag_ctxt.print_all(&mut std::io::stderr()).unwrap();

    let ast = Program {
        items: vec![Item::Function(Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            body: Block {
                stmts: vec![
                    Stmt::VarDef {
                        name: "x".to_string(),
                        ty: None,
                        init: Some(ExprNode {
                            span: Span {
                                low: 0.into(),
                                high: 0.into(),
                            },
                            expr: Expr::Int(42),
                        }),
                    },
                    Stmt::VarDef {
                        name: "y".to_string(),

                        ty: None,

                        init: Some(ExprNode {
                            span: Span {
                                low: 0.into(),
                                high: 0.into(),
                            },

                            expr: Expr::Float(3.14),
                        }),
                    },
                    Stmt::VarDef {
                        name: "z".to_string(),

                        ty: None,

                        init: Some(ExprNode {
                            span: Span {
                                low: 0.into(),
                                high: 0.into(),
                            },

                            expr: Expr::StringLiteral("Hello, world!".to_string()),
                        }),
                    },
                    Stmt::Return(Some(ExprNode {
                        span: Span {
                            low: 0.into(),
                            high: 0.into(),
                        },

                        expr: Expr::Variable("x".to_string()),
                    })),
                ],
            },
        })],
    };
    print!("{:#?}", ast);
}
