use std::fmt::format;
use std::iter::Peekable;
use crate::common::{DiagCtxt, span};
use crate::frontend::lexer::{
    Lexer, token::{Token, TokenKind}
};
use crate::frontend::ast::{
    Program, item::*, expr::*, stmt::*, typ::*
};

pub struct Parser<'a, 'diag> {
    tokens: Peekable<Lexer<'a>>,
    diag: &'diag mut DiagCtxt,
}

impl<'a, 'diag> Parser<'a, 'diag> {
    pub fn new(lexer: Lexer<'a>, diag: &'diag mut DiagCtxt) -> Self {
        Self { tokens: lexer.peekable(), diag }
    }

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    fn advance(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        match self.peek() {
            Some(token) => token.kind == kind,
            None => false,
        }
    }

    fn consume(&mut self, kind: TokenKind, msg: &str) -> Option<Token> {
        if self.check(kind) {
            return self.advance();
        }
        let span = self.peek().map(|t|t.span).unwrap_or(span::Span::new(0.into(), 0.into()));
        let diagnostic = self.diag.error(span, msg).note(format!("Expected {:?}", kind)).build();
        self.diag.emit(diagnostic);
        None
    }

    fn is_at_end(&mut self) -> bool {
        match self.peek() {
            Some(token) => token.kind == TokenKind::TokenEOF,
            None => true,
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item());
        }
        Program { items }
    }

    fn parse_item(&mut self) -> Item {
        // 解析函数、变量等
        match self.peek() {
            Some(token) => {
                match token.kind {
                    TokenKind::Decl => Item::FunctionDecl(self.parse_header()),
                    TokenKind::Def => Item::FunctionDef(self.parse_function()),
                    TokenKind::Class => Item::Class(self.parse_class()),
                    // TODO: global variable 
                    // TokenKind::Var => self.parse_var_def_stmt(),
                    _ => panic!("Unexpected token: {:?}", token),
                }
            }
            None => panic!("Unexpected end of input")
        }
    }

    fn parse_stmt(&mut self) -> Stmt {
        // 解析语句
        match self.peek() {
            Some(token) => {
                match token.kind {
                    TokenKind::If => self.parse_if_stmt(),
                    TokenKind::While => self.parse_while_stmt(),
                    TokenKind::Return => self.parse_return_stmt(),
                    TokenKind::Var => self.parse_var_def_stmt(),
                    _ => {
                        let expr = self.parse_expr();
                        self.consume(TokenKind::Semicolon, "Expected ';' after expression.");
                        Stmt::ExprStmt(expr)
                    }
                }
                
            }
            None => panic!("Unexpected end of input")
        }
    }

    fn parse_expr(&mut self) -> ExprNode {
        // 解析表达式
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ExprNode {
        // 解析赋值表达式
        let left = self.parse_logical_or();
        if self.check(TokenKind::Equal) {
            self.advance(); // consume '='
            let right = self.parse_assignment();
            ExprNode {
                span: left.span.merge(right.span),
                expr: Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(right),
                },
            }
        } else {
            left
        }
    }

    fn parse_logical_or(&mut self) -> ExprNode {
        // 解析逻辑或表达式
        let expr = self.parse_logical_and();
        if self.check(TokenKind::Or) {
            self.advance(); // consume 'or'
            let right = self.parse_logical_or();
            ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: BinaryOp::Or,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            }
        } else {
            expr
        }
    }

    fn parse_logical_and(&mut self) -> ExprNode {
        // 解析逻辑与表达式
        let expr = self.parse_equality();
        if self.check(TokenKind::And) {
            self.advance(); // consume 'and'
            let right = self.parse_logical_and();
            ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: BinaryOp::And,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            }
        } else {
            expr
        }
    }

    fn parse_equality(&mut self) -> ExprNode {
        // 解析相等表达式
        let mut expr = self.parse_comparison();
        while self.check(TokenKind::EqualEqual) || self.check(TokenKind::BangEqual) {
            let op = self.advance().unwrap(); // consume '==' or '!='
            let right = self.parse_comparison();
            let kind = match op.kind {
                TokenKind::EqualEqual => BinaryOp::Eq,
                TokenKind::BangEqual => BinaryOp::NotEq,
                _ => unreachable!(),
            };
            expr = ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: kind,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            };
        }
        expr
    }

    fn parse_comparison(&mut self) -> ExprNode {
        // 解析比较表达式
        let mut expr = self.parse_term();
        while self.check(TokenKind::Less) || self.check(TokenKind::Greater) {
            let op = self.advance().unwrap(); // consume '<' or '>'
            let right = self.parse_term();
            let kind = match op.kind {
                TokenKind::Less => BinaryOp::Lt,
                TokenKind::Greater => BinaryOp::Gt,
                _ => unreachable!(),
            };
            expr = ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: kind,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            };
        }
        expr
    }

    fn parse_term(&mut self) -> ExprNode {
        // 解析加减表达式
        let mut expr = self.parse_factor();
        while self.check(TokenKind::Plus) || self.check(TokenKind::Minus) {
            let op = self.advance().unwrap(); // consume '+' or '-'
            let kind = match op.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),    
            };
            let right = self.parse_factor();
            expr = ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: kind,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            };
        }
        expr
    }

    fn parse_factor(&mut self) -> ExprNode {
        // 解析乘除表达式
        let mut expr = self.parse_unary();
        while self.check(TokenKind::Star) || self.check(TokenKind::Slash) {
            let op = self.advance().unwrap(); // consume '*' or '/'
            let right = self.parse_unary();
            let kind = match op.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            expr = ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: kind,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            };
        }
        expr
    }

    fn parse_unary(&mut self) -> ExprNode {
        // 解析一元表达式
        if self.check(TokenKind::Bang) || 
           self.check(TokenKind::Minus) || 
           self.check(TokenKind::BitwiseAnd) || 
           self.check(TokenKind::Star) {
            let op = self.advance().unwrap(); // consume '!' or '-' or '&' or '*'
            let expr = self.parse_unary();
            return ExprNode {
                span: expr.span,
                expr: Expr::Unary {
                    op: match op.kind {
                        TokenKind::Bang => UnaryOp::Not,
                        TokenKind::Minus => UnaryOp::Neg,
                        TokenKind::BitwiseAnd => UnaryOp::AddrOf,
                        TokenKind::Star => UnaryOp::Deref,
                        _ => unreachable!(),
                    },
                    expr: Box::new(expr),
                },
            };
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> ExprNode {
        // 解析后缀表达式
        let mut expr = self.parse_primary();
        loop {
            let token = self.peek().unwrap();
            match token.kind {
                TokenKind::LeftParen => {
                    expr =  self.finish_call(expr);
                }
                TokenKind::Dot => {
                    self.advance(); // consume '.'
                    let field_token = self.consume(TokenKind::Identifier, "Expected field name after '.'.");
                    expr =  ExprNode {
                        span: expr.span,
                        expr: Expr::Member {
                            object: Box::new(expr),
                            field: field_token.unwrap().lexeme,
                        },
                    };
                }
                TokenKind::PlusPlus => {
                    self.advance(); // consume '++'
                    expr =  ExprNode {
                        span: expr.span,
                        expr: Expr::Unary {
                            op: UnaryOp::Inc,
                            expr: Box::new(expr),
                        },
                    };
                }
                TokenKind::MinusMinus => {
                    self.advance(); // consume '--'
                    expr =  ExprNode {
                        span: expr.span,
                        expr: Expr::Unary {
                            op: UnaryOp::Dec,
                            expr: Box::new(expr),
                        },
                    };
                }
                TokenKind::LeftBracket => {
                    self.advance(); // consume '['
                    let index = self.parse_expr();
                    self.consume(TokenKind::RightBracket, "Expected ']' after index expression.");
                    expr =  ExprNode {
                        span: expr.span,
                        expr: Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                _ => break,
                
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> ExprNode {
        // 解析基本表达式
        let token = self.advance().expect("Unexpected end of input");
        match token.kind {
            TokenKind::IntLiteral => ExprNode {
                span: token.span,
                expr: Expr::Int(token.lexeme.parse().unwrap()),
            },
            TokenKind::FloatLiteral => ExprNode {
                span: token.span,
                expr: Expr::Float(token.lexeme.parse().unwrap()),
            },
            TokenKind::True => ExprNode {
                span: token.span,
                expr: Expr::Bool(true),
            },
            TokenKind::False => ExprNode {
                span: token.span,
                expr: Expr::Bool(false),
            },
            TokenKind::CharLiteral => ExprNode {
                span: token.span,
                expr: Expr::Char(token.lexeme.chars().next().unwrap()),
            },
            TokenKind::StringLiteral => ExprNode {
                span: token.span,
                expr: Expr::StringLiteral(token.lexeme),
            },
            TokenKind::Identifier => ExprNode {
                span: token.span,
                expr: Expr::Variable(token.lexeme),
            },
            TokenKind::LeftParen => {
                let expr = self.parse_expr();
                self.consume(TokenKind::RightParen, "Expected ')' after expression.");
                expr
            },
            TokenKind::LeftBracket => {
                // 解析列表字面量
                let mut elements = Vec::new();
                if !self.check(TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expr());
                        if self.check(TokenKind::Comma) {
                            self.advance(); // consume ','
                        } else {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RightBracket, "Expected ']' after list literal.");
                ExprNode {
                    span: token.span,
                    expr: Expr::List { elements },
                }
            }
            _ => panic!("Unexpected token: {:?}", token),
        }
    }

    fn parse_if_stmt(&mut self) -> Stmt {
        // 解析 if 语句
        self.consume(TokenKind::If, "Expected 'if' at the beginning of if statement.");
        let condition = self.parse_expr();
        let then_branch = self.parse_block();
        let mut elif_branches = Vec::new();
        while self.check(TokenKind::Elif) {
            self.advance(); // consume 'elif'
            let elif_condition = self.parse_expr();
            let elif_branch = self.parse_block();
            elif_branches.push((elif_condition, elif_branch));
        }
        let else_branch = if self.check(TokenKind::Else) {
            self.advance(); // consume 'else'
            self.parse_block()
        } else {
            Block { stmts: Vec::new() }
        };
        Stmt::If {
            condition: condition,
            then_branch: then_branch,
            elif_branch: elif_branches,
            else_branch: else_branch,
        }
    }

    fn parse_while_stmt(&mut self) -> Stmt {
        // 解析 while 语句
        self.consume(TokenKind::While, "Expected 'while' at the beginning of while statement.");
        let condition = self.parse_expr();
        let body = self.parse_block();
        Stmt::Loop {
            condition: condition,
            body: body,
        }
    }

    fn parse_return_stmt(&mut self) -> Stmt {
        // 解析 return 语句
        self.consume(TokenKind::Return, "Expected 'return' at the beginning of return statement.");
        let expr = if !self.check(TokenKind::Semicolon) {
            Some(self.parse_expr())
        } else {
            None
        };
        self.consume(TokenKind::Semicolon, "Expected ';' after return statement.");
        Stmt::Return(expr)
    }

    fn parse_var_def_stmt(&mut self) -> Stmt {
        // 解析变量定义语句
        self.consume(TokenKind::Var, "Expected 'var' at the beginning of variable definition.");
        let var_name = self.consume(TokenKind::Identifier, "Expected variable name.").unwrap().lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after variable name.");
        let var_type = self.parse_type();
        let var_init = if self.check(TokenKind::Equal) {
            self.advance(); // consume '='
            Some(self.parse_expr())
        } else {
            None
        };
        self.consume(TokenKind::Semicolon, "Expected ';' after variable definition.");
        Stmt::VarDef {
            name: var_name,
            ty: var_type,
            init: var_init,
        }

    }

    fn parse_block(&mut self) -> Block {
        // 解析代码块
        self.consume(TokenKind::LeftBrace, "Expected '{' at the beginning of block.");
        let mut stmts = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            stmts.push(self.parse_stmt());
        }
        self.consume(TokenKind::RightBrace, "Expected '}' at the end of block.");
        Block { stmts }
    }

    fn parse_type(&mut self) -> Type {
        // 解析类型
        match self.peek() {
            Some(token) => {
                match token.kind {
                    TokenKind::Int => {
                        self.advance().unwrap();
                        Type::Int
                    }
                    TokenKind::Float => {
                        self.advance().unwrap();
                        Type::Float
                    }
                    TokenKind::Bool => {
                        self.advance().unwrap();
                        Type::Bool
                    }
                    TokenKind::Char => {
                        self.advance().unwrap();
                        Type::Char
                    }
                    TokenKind::Str => {
                        self.advance().unwrap();
                        Type::Str
                    }
                    TokenKind::None => {
                        self.advance().unwrap();
                        Type::Void
                    }
                    TokenKind::Identifier => {
                        let class_name = self.advance().unwrap().lexeme;
                        Type::Class(class_name)
                    }
                    TokenKind::Ptr => {
                        self.advance().unwrap(); // consume 'ptr'
                        self.consume(TokenKind::Less, "Expected '<' after 'ptr'.");
                        let inner_type = self.parse_type();
                        self.consume(TokenKind::Greater, "Expected '>' after pointer inner type.");
                        Type::Ptr(Box::new(inner_type))
                    }
                    TokenKind::List => {
                        self.advance().unwrap(); // consume 'list'
                        self.consume(TokenKind::Less, "Expected '<' after 'list'.");
                        let element_type = self.parse_type();
                        self.consume(TokenKind::Greater, "Expected '>' after list element type.");
                        Type::List(Box::new(element_type))
                    }
                    TokenKind::Array => {
                        self.advance().unwrap(); // consume 'array'
                        self.consume(TokenKind::Less, "Expected '<' after 'array'.");
                        let size_token = self.consume(TokenKind::IntLiteral, "Expected array size as an integer literal.");
                        let size = size_token.unwrap().lexeme.parse::<usize>().expect("Array size must be a valid integer.");
                        self.consume(TokenKind::Comma, "Expected ',' after array size.");
                        let element_type = self.parse_type();
                        self.consume(TokenKind::Greater, "Expected '>' after array element type.");
                        Type::Array(size, Box::new(element_type))
                    }
                    _ => panic!("Unexpected token in type expression"),
                }
            }
            None => panic!("Unexpected end of input")
        }
    }

    fn parse_parameter(&mut self) -> Option<Parameter> {
        // 解析函数参数
        let param_name = self.consume(TokenKind::Identifier, "Expected parameter name.").unwrap().lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after parameter name.");
        let param_type = self.parse_type();
        Some(Parameter { name: param_name, ty: param_type })
    }

    fn parse_header(&mut self) -> FunctionDecl {
        // 解析函数头部，返回函数名、参数列表和返回类型
        let func_name = self.consume(TokenKind::Identifier, "Expected function name.").unwrap().lexeme;
        self.consume(TokenKind::LeftParen, "Expected '(' after function name.");
        let mut params = Vec::new();
        while !self.check(TokenKind::RightParen) {
            params.push(self.parse_parameter().expect("Expected parameter."));
            if !self.check(TokenKind::RightParen) {
                self.consume(TokenKind::Comma, "Expected ',' between parameters.");
            }
        }
        self.consume(TokenKind::RightParen, "Expected ')' after parameters.");

        let return_type = if self.check(TokenKind::Arrow) {
            self.advance(); // consume '->'
            Some(self.parse_type())
        } else {
            None
        };
        FunctionDecl { name: func_name, params, return_type: return_type }
    }

    fn parse_function(&mut self) -> FunctionDef {
        // 解析函数定义
        self.consume(TokenKind::Def, "Expected 'def' before function definition.");
        let header = self.parse_header();
        let body = self.parse_block();
        FunctionDef {
            function_header: header,
            body,
        }
    }

    fn parse_fields(&mut self) -> Field {
        // 解析类的字段定义
        self.consume(TokenKind::Var, "Expected 'var' before field definition.");
        let field_name = self.consume(TokenKind::Identifier, "Expected field name.").unwrap().lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after field name.");
        let field_type = self.parse_type();
        self.consume(TokenKind::Semicolon, "Expected ';' after field definition.");
        Field {
            name: field_name,
            ty: field_type,
        }
    }

    fn parse_class(&mut self) -> Class {
        // 解析类定义
        self.consume(TokenKind::Class, "Expected 'class' before class definition.");
        let class_name = self.consume(TokenKind::Identifier, "Expected class name.").unwrap().lexeme;
        self.consume(TokenKind::LeftBrace, "Expected '{' after class name.");
        let mut fields = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            fields.push(self.parse_fields());
        }
        self.consume(TokenKind::RightBrace, "Expected '}' after class definition.");
        Class {
            name: class_name,
            fields: fields,
        }
    }

    fn parse_arguments(&mut self) -> Vec<ExprNode> {
        // 解析函数调用的参数列表
        let mut args = Vec::new();
        if !self.check(TokenKind::RightParen) {
            loop {
                args.push(self.parse_expr());
                if self.check(TokenKind::Comma) {
                    self.advance(); // consume ','
                } else {
                    break;
                }
            }
        }
        args
    }

    fn finish_call(&mut self, callee: ExprNode) -> ExprNode {
        // 解析函数调用
        self.consume(TokenKind::LeftParen, "Expected '(' after callee.");
        let args = self.parse_arguments();
        self.consume(TokenKind::RightParen, "Expected ')' after arguments.");
        ExprNode {
            span: callee.span,
            expr: Expr::Call {
                callee: Box::new(callee),
                args,
            },
        }
    }
}

