use crate::common::DiagCtxt;
use crate::common::error::{ParseError, ParseResult};
use crate::common::span::Span;
use crate::frontend::ast::{Program, expr::*, item::*, stmt::*, typ::*};
use crate::frontend::lexer::{
    Lexer,
    token::{Token, TokenKind},
};
use std::iter::Peekable;

pub struct Parser<'a, 'diag> {
    tokens: Peekable<Lexer<'a>>,
    diag: &'diag mut DiagCtxt,
    last_span: Span,
}

impl<'a, 'diag> Parser<'a, 'diag> {
    pub fn new(lexer: Lexer<'a>, diag: &'diag mut DiagCtxt) -> Self {
        let zero = Span::new(0.into(), 0.into());
        Self {
            tokens: lexer.peekable(),
            diag,
            last_span: zero,
        }
    }

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    fn advance(&mut self) -> ParseResult<Token> {
        match self.tokens.next() {
            Some(tok) => {
                self.last_span = tok.span;
                Ok(tok)
            }
            None => {
                let span = self.last_span;
                self.err(span, "Unexpected end of file.")
            }
        }
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        self.peek().map_or(false, |t| t.kind == kind)
    }

    fn check_any(&mut self, kinds: &[TokenKind]) -> bool {
        self.peek().map_or(true, |t| kinds.contains(&t.kind))
    }

    fn consume(&mut self, kind: TokenKind, msg: &str) -> ParseResult<Token> {
        if self.check(kind) {
            return self.advance();
        }
        let span = self.current_span();
        println!("{}", span);

        let err = self
            .diag
            .error(span, msg)
            .note(format!(
                "expected {:?}, found {:?}",
                kind,
                self.peek().map(|t| t.kind)
            ))
            .build();
        self.diag.emit(err);
        Err(ParseError::new(span))
    }

    fn is_at_end(&mut self) -> bool {
        match self.peek() {
            Some(token) => token.kind == TokenKind::TokenEOF,
            None => true,
        }
    }

    fn current_span(&mut self) -> Span {
        self.peek().map(|x| x.span).unwrap_or(self.last_span)
    }

    fn err<T>(&mut self, span: Span, msg: impl Into<String>) -> ParseResult<T> {
        let d = self.diag.error(span, msg).build();
        self.diag.emit(d);
        Err(ParseError::new(span))
    }

    fn unexpected<T>(&mut self, context: &str) -> ParseResult<T> {
        let span = self.current_span();
        let msg = match self.peek() {
            Some(t) => format!("unexpected token {:?} {}", t.kind, context),
            None => format!("unexpected end of file {}", context),
        };
        self.err(span, msg)
    }

    fn synchronize(&mut self) {
        // If the offending token is a semicolon, consume it and stop.
        // Otherwise keep skipping until we see something that looks
        // like the beginning of a new statement or declaration.
        loop {
            if self.is_at_end() {
                return;
            }
            // Stop *after* a semicolon — the next token is fresh.
            if self
                .peek()
                .map_or(false, |t| t.kind == TokenKind::Semicolon)
            {
                let _ = self.advance();
                return;
            }
            // Stop *before* keywords that begin new constructs.
            if self.check_any(&[
                TokenKind::Let,
                TokenKind::Def,
                TokenKind::Class,
                TokenKind::If,
                TokenKind::While,
                TokenKind::Return,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::TokenEOF,
            ]) {
                return;
            }
            let _ = self.advance();
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        while !self.is_at_end() {
            match self.parse_item() {
                Ok(decl) => items.push(decl),
                Err(_) => {
                    self.synchronize();
                }
            }
        }
        Program { items }
    }

    fn parse_item(&mut self) -> ParseResult<ItemNode> {
        // 解析函数、变量等
        match self.peek().map(|t| t.kind) {
            Some(TokenKind::Decl) => Ok(ItemNode { span: self.current_span(), item: Item::FunctionDecl(self.parse_func_decl()?)}),
            Some(TokenKind::Def) => Ok(ItemNode { span: self.current_span(), item: Item::FunctionDef(self.parse_function()?)}),
            Some(TokenKind::Class) => Ok(ItemNode { span: self.current_span(), item: Item::Class(self.parse_class()?)}),
            Some(TokenKind::Trait) => Ok(ItemNode { span: self.current_span(), item: Item::Trait(self.parse_trait()?)}),
            Some(TokenKind::Impl) => Ok(ItemNode { span: self.current_span(), item: Item::Impl(self.parse_impl()?)}),
            // TODO: global variable
            // TokenKind::Let => self.parse_var_def_stmt(),
            _ => self.unexpected("Expected 'let', 'def', 'decl', 'class', 'trait', 'impl'."),
        }
    }

    fn parse_stmt(&mut self) -> ParseResult<StmtNode> {
        // 解析语句
        match self.peek() {
            Some(token) => match token.kind {
                TokenKind::If => self.parse_if_stmt(),
                TokenKind::While => self.parse_while_stmt(),
                TokenKind::Return => self.parse_return_stmt(),
                TokenKind::Let => self.parse_var_def_stmt(),
                _ => {
                    let expr = self.parse_expr()?;
                    self.consume(TokenKind::Semicolon, "Expected ';' after expression.")?;
                    Ok(StmtNode { span: expr.span, stmt: Stmt::ExprStmt(expr) })
                }
            },
            None => self.unexpected("Unexpected end of input"),
        }
    }

    fn parse_expr(&mut self) -> ParseResult<ExprNode> {
        // 解析表达式
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ParseResult<ExprNode> {
        // 解析赋值表达式
        let left = self.parse_logical_or()?;
        if self.check(TokenKind::Equal) {
            self.advance()?; // consume '='
            let right = self.parse_assignment()?;
            Ok(ExprNode {
                span: left.span.merge(right.span),
                expr: Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(right),
                },
            })
        } else {
            Ok(left)
        }
    }

    fn parse_logical_or(&mut self) -> ParseResult<ExprNode> {
        // 解析逻辑或表达式
        let expr = self.parse_logical_and()?;
        if self.check(TokenKind::Or) {
            self.advance()?; // consume 'or'
            let right = self.parse_logical_or()?;
            Ok(ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: BinaryOp::Or,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_and(&mut self) -> ParseResult<ExprNode> {
        // 解析逻辑与表达式
        let expr = self.parse_equality()?;
        if self.check(TokenKind::And) {
            self.advance()?; // consume 'and'
            let right = self.parse_logical_and()?;
            Ok(ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: BinaryOp::And,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_equality(&mut self) -> ParseResult<ExprNode> {
        // 解析相等表达式
        let mut expr = self.parse_comparison()?;
        while self.check(TokenKind::EqualEqual) || self.check(TokenKind::BangEqual) {
            let op = self.advance().unwrap(); // consume '==' or '!='
            let right = self.parse_comparison()?;
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
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> ParseResult<ExprNode> {
        // 解析比较表达式
        let mut expr = self.parse_term()?;
        while self.check(TokenKind::Less) || self.check(TokenKind::Greater) {
            let op = self.advance().unwrap(); // consume '<' or '>'
            let right = self.parse_term()?;
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
        Ok(expr)
    }

    fn parse_term(&mut self) -> ParseResult<ExprNode> {
        // 解析加减表达式
        let mut expr = self.parse_factor()?;
        while self.check(TokenKind::Plus) || self.check(TokenKind::Minus) {
            let op = self.advance().unwrap(); // consume '+' or '-'
            let kind = match op.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            expr = ExprNode {
                span: expr.span.merge(right.span),
                expr: Expr::Binary {
                    op: kind,
                    lhs: Box::new(expr),
                    rhs: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> ParseResult<ExprNode> {
        // 解析乘除表达式
        let mut expr = self.parse_unary()?;
        while self.check(TokenKind::Star) || self.check(TokenKind::Slash) {
            let op = self.advance().unwrap(); // consume '*' or '/'
            let right = self.parse_unary()?;
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
        Ok(expr)
    }

    fn parse_unary(&mut self) -> ParseResult<ExprNode> {
        // 解析一元表达式
        if self.check(TokenKind::Bang)
            || self.check(TokenKind::Minus)
            || self.check(TokenKind::BitwiseAnd)
            || self.check(TokenKind::Star)
        {
            let op = self.advance()?; // consume '!' or '-' or '&' or '*'
            let expr = self.parse_unary()?;
            return Ok(ExprNode {
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
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> ParseResult<ExprNode> {
        // 解析后缀表达式
        let mut expr = self.parse_primary()?;
        loop {
            let token = self.peek().unwrap();
            match token.kind {
                TokenKind::LeftParen => {
                    expr = self.finish_call(expr)?;
                }
                TokenKind::Dot => {
                    self.advance()?; // consume '.'
                    let field_token =
                        self.consume(TokenKind::Identifier, "Expected field name after '.'.");
                    expr = ExprNode {
                        span: expr.span,
                        expr: Expr::Member {
                            object: Box::new(expr),
                            field: field_token.unwrap().lexeme,
                        },
                    };
                }
                TokenKind::PlusPlus => {
                    self.advance()?; // consume '++'
                    expr = ExprNode {
                        span: expr.span,
                        expr: Expr::Unary {
                            op: UnaryOp::Inc,
                            expr: Box::new(expr),
                        },
                    };
                }
                TokenKind::MinusMinus => {
                    self.advance()?; // consume '--'
                    expr = ExprNode {
                        span: expr.span,
                        expr: Expr::Unary {
                            op: UnaryOp::Dec,
                            expr: Box::new(expr),
                        },
                    };
                }
                TokenKind::LeftBracket => {
                    self.advance()?; // consume '['
                    let index = self.parse_expr()?;
                    self.consume(
                        TokenKind::RightBracket,
                        "Expected ']' after index expression.",
                    )?;
                    expr = ExprNode {
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
        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<ExprNode> {
        // 解析基本表达式
        let token = self.advance().expect("Unexpected end of input");
        match token.kind {
            TokenKind::IntLiteral => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::Int(token.lexeme.parse().unwrap())),
            }),
            TokenKind::FloatLiteral => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::Float(token.lexeme.parse().unwrap())),
            }),
            TokenKind::True => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::Bool(true)),
            }),
            TokenKind::False => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::Bool(false)),
            }),
            TokenKind::CharLiteral => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::Char(token.lexeme.chars().next().unwrap())),
            }),
            TokenKind::StringLiteral => Ok(ExprNode {
                span: token.span,
                expr: Expr::Literal(Literal::StringLiteral(token.lexeme)),
            }),
            TokenKind::Identifier => Ok(ExprNode {
                span: token.span,
                expr: Expr::Variable(token.lexeme),
            }),
            TokenKind::LeftParen => {
                let expr = self.parse_expr()?;
                self.consume(TokenKind::RightParen, "Expected ')' after expression.")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                // 解析列表字面量
                let mut elements = Vec::new();
                if !self.check(TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if self.check(TokenKind::Comma) {
                            self.advance()?; // consume ','
                        } else {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RightBracket, "Expected ']' after list literal.")?;
                Ok(ExprNode {
                    span: token.span,
                    expr: Expr::List { elements },
                })
            }
            _ => panic!("Unexpected token: {:?}", token),
        }
    }

    fn parse_if_stmt(&mut self) -> ParseResult<StmtNode> {
        // 解析 if 语句
        self.consume(
            TokenKind::If,
            "Expected 'if' at the beginning of if statement.",
        )?;
        let condition = self.parse_expr()?;
        let then_branch = self.parse_block()?;
        let mut elif_branches = Vec::new();
        while self.check(TokenKind::Elif) {
            self.advance()?; // consume 'elif'
            let elif_condition = self.parse_expr()?;
            let elif_branch = self.parse_block()?;
            elif_branches.push((elif_condition, elif_branch));
        }
        let else_branch = if self.check(TokenKind::Else) {
            self.advance()?; // consume 'else'
            self.parse_block()?
        } else {
            Block { stmts: Vec::new() }
        };
        Ok(StmtNode { span: self.current_span(), 
            stmt: Stmt::If {
                condition: condition,
                then_branch: then_branch,
                elif_branch: elif_branches,
                else_branch: else_branch,
        } })
    }

    fn parse_while_stmt(&mut self) -> ParseResult<StmtNode> {
        // 解析 while 语句
        self.consume(
            TokenKind::While,
            "Expected 'while' at the beginning of while statement.",
        )?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(
            StmtNode { 
                span: self.current_span(), 
                stmt: Stmt::Loop {
                    condition: condition,
                    body: body,
                    }
                }
            )
    }

    fn parse_return_stmt(&mut self) -> ParseResult<StmtNode> {
        // 解析 return 语句
        let return_token = self.consume(
            TokenKind::Return,
            "Expected 'return' at the beginning of return statement.",
        )?;
        let expr = if !self.check(TokenKind::Semicolon) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume(TokenKind::Semicolon, "Expected ';' after return statement.")?;
        Ok(StmtNode { span: return_token.span, stmt: Stmt::Return(expr) })
    }

    fn parse_var_def_stmt(&mut self) -> ParseResult<StmtNode> {
        // 解析变量定义语句
        self.consume(
            TokenKind::Let,
            "Expected 'let' at the beginning of variable definition.",
        )?;
        let var_name = self.consume(TokenKind::Identifier, "Expected variable name.")?;
        let var_name = var_name.lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after variable name.")?;
        let var_type = self.parse_type()?;
        let var_init = if self.check(TokenKind::Equal) {
            self.advance()?; // consume '='
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume(
            TokenKind::Semicolon,
            "Expected ';' after variable definition.",
        )?;
        Ok(StmtNode { 
                span: self.current_span(), 
                stmt: Stmt::VarDef {
                    name: var_name,
                    ty: var_type,
                    init: var_init,
        } })
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        // 解析代码块
        self.consume(
            TokenKind::LeftBrace,
            "Expected '{' at the beginning of block.",
        )?;
        let mut stmts = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.consume(TokenKind::RightBrace, "Expected '}' at the end of block.")?;
        Ok(Block { stmts })
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        // 解析类型
        match self.peek() {
            Some(token) => {
                match token.kind {
                    TokenKind::Int => {
                        self.advance()?;
                        Ok(Type::Int)
                    }
                    TokenKind::Float => {
                        self.advance()?;
                        Ok(Type::Float)
                    }
                    TokenKind::Bool => {
                        self.advance()?;
                        Ok(Type::Bool)
                    }
                    TokenKind::Char => {
                        self.advance()?;
                        Ok(Type::Char)
                    }
                    TokenKind::Str => {
                        self.advance()?;
                        Ok(Type::Str)
                    }
                    TokenKind::None => {
                        self.advance()?;
                        Ok(Type::Void)
                    }
                    TokenKind::Identifier => {
                        let class_name = self.advance()?;
                        let class_name = class_name.lexeme;
                        Ok(Type::Class(class_name))
                    }
                    TokenKind::Ptr => {
                        self.advance().unwrap(); // consume 'ptr'
                        self.consume(TokenKind::Less, "Expected '<' after 'ptr'.")?;
                        let inner_type = self.parse_type()?;
                        self.consume(TokenKind::Greater, "Expected '>' after pointer inner type.")?;
                        Ok(Type::Ptr(Box::new(inner_type)))
                    }
                    TokenKind::List => {
                        self.advance()?; // consume 'list'
                        self.consume(TokenKind::Less, "Expected '<' after 'list'.")?;
                        let element_type = self.parse_type()?;
                        self.consume(TokenKind::Greater, "Expected '>' after list element type.")?;
                        Ok(Type::List(Box::new(element_type)))
                    }
                    TokenKind::Array => {
                        self.advance()?; // consume 'array'
                        self.consume(TokenKind::Less, "Expected '<' after 'array'.")?;
                        let size_token = self.consume(
                            TokenKind::IntLiteral,
                            "Expected array size as an integer literal.",
                        )?;
                        let size = size_token
                            .lexeme
                            .parse::<usize>()
                            .expect("Array size must be a valid integer.");
                        self.consume(TokenKind::Comma, "Expected ',' after array size.")?;
                        let element_type = self.parse_type()?;
                        self.consume(TokenKind::Greater, "Expected '>' after array element type.")?;
                        Ok(Type::Array(size, Box::new(element_type)))
                    }
                    _ => self.unexpected("Expected type name."),
                }
            }
            None => self.unexpected("Expected end of file."),
        }
    }

    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        // 解析函数参数
        let param_name = self.consume(TokenKind::Identifier, "Expected parameter name.")?;
        let param_name = param_name.lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after parameter name.")?;
        let param_type = self.parse_type()?;
        Ok(Parameter {
            name: param_name,
            ty: param_type,
        })
    }

    fn parse_header(&mut self) -> ParseResult<FunctionDecl> {
        // 解析函数头部，返回函数名、参数列表和返回类型
        let func_name = self.consume(TokenKind::Identifier, "Expected function name.")?;
        let func_name = func_name.lexeme;
        self.consume(TokenKind::LeftParen, "Expected '(' after function name.")?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RightParen) {
            params.push(self.parse_parameter()?);
            if !self.check(TokenKind::RightParen) {
                self.consume(TokenKind::Comma, "Expected ',' between parameters.")?;
            }
        }
        self.consume(TokenKind::RightParen, "Expected ')' after parameters.")?;

        let return_type = if self.check(TokenKind::Arrow) {
            self.advance()?; // consume '->'
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(FunctionDecl {
            name: func_name,
            params,
            return_type: return_type,
        })
    }

    fn parse_func_decl(&mut self) -> ParseResult<FunctionDecl> {
        // 解析函数声明
        self.consume(
            TokenKind::Decl,
            "Expected 'decl' before function declaration.",
        )?;
        let header = self.parse_header();
        self.consume(
            TokenKind::Semicolon,
            "Expected ';' after function declaration.",
        )?;
        header
    }

    fn parse_function(&mut self) -> ParseResult<FunctionDef> {
        // 解析函数定义
        self.consume(TokenKind::Def, "Expected 'def' before function definition.")?;
        let header = self.parse_header()?;
        let body = self.parse_block()?;
        Ok(FunctionDef {
            function_header: header,
            body,
        })
    }

    fn parse_trait(&mut self) -> ParseResult<Trait> {
        // 解析 trait 块
        self.consume(TokenKind::Trait, "Expected 'trait' before trait block.")?;
        let trait_name =
            self.consume(TokenKind::Identifier, "Expected trait name after 'trait'.")?;
        let trait_name = trait_name.lexeme;
        self.consume(TokenKind::LeftBrace, "Expected '{' after trait name.")?;
        let mut methods = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            methods.push(self.parse_func_decl()?);
        }
        self.consume(TokenKind::RightBrace, "Expected '}' after trait methods.")?;
        Ok(Trait {
            name: trait_name,
            methods: methods,
        })
    }

    fn parse_impl(&mut self) -> ParseResult<Impl> {
        // 解析 impl 块
        self.consume(TokenKind::Impl, "Expected 'impl' before impl block.")?;
        // TODO: parse trait name if needed
        let mut trait_or_class_tok = self.consume(
            TokenKind::Identifier,
            "Expected class or trait name after 'impl'.",
        )?;
        let mut trait_or_class_name = trait_or_class_tok.lexeme;
        let mut trait_name: Option<String> = None;
        let mut methods = Vec::new();
        if self.check(TokenKind::For) {
            self.advance()?; // consume 'for'
            trait_name = Some(trait_or_class_name);
            trait_or_class_tok =
                self.consume(TokenKind::Identifier, "Expected class name after 'for'.")?;
            trait_or_class_name = trait_or_class_tok.lexeme;
        }
        self.consume(TokenKind::LeftBrace, "Expected '{' after class name.")?;
        while !self.check(TokenKind::RightBrace) {
            methods.push(self.parse_function()?);
        }
        self.consume(TokenKind::RightBrace, "Expected '}' after methods.")?;
        Ok(Impl {
            trait_name: trait_name,
            class_name: trait_or_class_name,
            methods: methods,
        })
    }

    fn parse_fields(&mut self) -> ParseResult<Field> {
        // 解析类的字段定义
        self.consume(TokenKind::Let, "Expected 'let' before field definition.")?;
        let field_name = self.consume(TokenKind::Identifier, "Expected field name.")?;
        let field_name = field_name.lexeme;
        self.consume(TokenKind::Colon, "Expected ':' after field name.")?;
        let field_type = self.parse_type()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after field definition.")?;
        Ok(Field {
            name: field_name,
            ty: field_type,
        })
    }

    fn parse_class(&mut self) -> ParseResult<Class> {
        // 解析类定义
        self.consume(
            TokenKind::Class,
            "Expected 'class' before class definition.",
        )?;
        let class_name = self.consume(TokenKind::Identifier, "Expected class name.")?;
        let class_name = class_name.lexeme;
        self.consume(TokenKind::LeftBrace, "Expected '{' after class name.")?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            fields.push(self.parse_fields()?);
        }
        self.consume(
            TokenKind::RightBrace,
            "Expected '}' after class definition.",
        )?;
        Ok(Class {
            name: class_name,
            fields: fields,
        })
    }

    fn parse_arguments(&mut self) -> ParseResult<Vec<ExprNode>> {
        // 解析函数调用的参数列表
        let mut args = Vec::new();
        if !self.check(TokenKind::RightParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.check(TokenKind::Comma) {
                    self.advance()?; // consume ','
                } else {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn finish_call(&mut self, callee: ExprNode) -> ParseResult<ExprNode> {
        // 解析函数调用
        self.consume(TokenKind::LeftParen, "Expected '(' after callee.")?;
        let args = self.parse_arguments()?;
        self.consume(TokenKind::RightParen, "Expected ')' after arguments.")?;
        Ok(ExprNode {
            span: callee.span,
            expr: Expr::Call {
                callee: Box::new(callee),
                args,
            },
        })
    }
}
