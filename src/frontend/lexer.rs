pub mod token;
// frontend/lexer.rs
use crate::common::span::{BytePos, Span};
use crate::frontend::lexer::token::{Token, TokenKind};
use phf::phf_map;
use std::iter::Peekable;
use std::str::Chars;

static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    "and" => TokenKind::And,
    "or" => TokenKind::Or,
    "true" => TokenKind::True,
    "false" => TokenKind::False,

    "class" => TokenKind::Class,
    "super" => TokenKind::Super,
    "self" => TokenKind::Sself,
    "new" => TokenKind::New,
    "impl" => TokenKind::Impl,
    "trait" => TokenKind::Trait,
    "for" => TokenKind::For,

    "def" => TokenKind::Def,
    "var" => TokenKind::Var,
    "decl" => TokenKind::Decl,

    "if" => TokenKind::If,
    "else" => TokenKind::Else,
    "elif" => TokenKind::Elif,
    "while" => TokenKind::While,
    "break" => TokenKind::Break,
    "continue" => TokenKind::Continue,
    "return" => TokenKind::Return,

    "none" => TokenKind::None,
    "ref" => TokenKind::Ref,
    "ptr" => TokenKind::Ptr,

    "int" => TokenKind::Int,
    "float" => TokenKind::Float,
    "bool" => TokenKind::Bool,
    "char" => TokenKind::Char,
    "str" => TokenKind::Str,

    "list" => TokenKind::List,
    "array" => TokenKind::Array,


};

fn is_keyword(ident: &str) -> TokenKind {
    KEYWORDS
        .get(ident)
        .copied()
        .unwrap_or(TokenKind::Identifier)
}
pub struct Lexer<'a> {
    src: &'a str,
    chars: Peekable<Chars<'a>>,
    pos: usize,         // 当前字节偏移
    token_start: usize, // 当前 token 起始偏移
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.chars().peekable(),
            pos: 0,
            token_start: 0,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn peek_second(&mut self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next()
    }

    fn span(&self) -> Span {
        Span::new(BytePos(self.token_start), BytePos(self.pos))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.bump();
                }
                Some('/') if self.peek_second() == Some('/') => {
                    // 单行注释
                    self.bump(); // 跳过 '/'
                    self.bump(); // 跳过第二个 '/'
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_second() == Some('*') => {
                    // 块注释
                    self.bump(); // '/'
                    self.bump(); // '*'
                    let mut closed = false;
                    while let Some(ch) = self.bump() {
                        if ch == '*' && self.peek() == Some('/') {
                            self.bump(); // '/'
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        // 未闭合的块注释，可以生成错误，但词法分析器继续
                        // 这里简单处理，忽略
                    }
                }
                _ => break,
            }
        }
    }

    /// 返回下一个常规 token
    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        self.token_start = self.pos;
        let ch = match self.bump() {
            Some(c) => c,
            None => {
                return Token {
                    kind: TokenKind::TokenEOF,
                    span: Span::new(BytePos(self.pos), BytePos(self.pos)),
                    lexeme: String::new(),
                };
            }
        };

        match ch {
            '(' => self.make_token_with_char(TokenKind::LeftParen, ch),
            ')' => self.make_token_with_char(TokenKind::RightParen, ch),
            '{' => self.make_token_with_char(TokenKind::LeftBrace, ch),
            '}' => self.make_token_with_char(TokenKind::RightBrace, ch),
            '[' => self.make_token_with_char(TokenKind::LeftBracket, ch),
            ']' => self.make_token_with_char(TokenKind::RightBracket, ch),
            ',' => self.make_token_with_char(TokenKind::Comma, ch),
            ';' => self.make_token_with_char(TokenKind::Semicolon, ch),
            ':' => self.make_token_with_char(TokenKind::Colon, ch),
            '.' => self.make_token_with_char(TokenKind::Dot, ch),
            '^' => self.make_token_with_char(TokenKind::Caret, ch),
            '\\' => self.make_token_with_char(TokenKind::Backslash, ch),
            '+' => {
                if self.peek() == Some('+') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::PlusPlus, "++")
                } else {
                    self.make_token_with_char(TokenKind::Plus, ch)
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::Arrow, "->")
                } else if self.peek() == Some('-') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::MinusMinus, "--")
                } else {
                    self.make_token_with_char(TokenKind::Minus, ch)
                }
            }
            '*' => self.make_token_with_char(TokenKind::Star, ch),
            '&' => self.make_token_with_char(TokenKind::BitwiseAnd, ch),
            '/' => self.make_token_with_char(TokenKind::Slash, ch),
            '%' => self.make_token_with_char(TokenKind::Modulo, ch),
            '=' => {
                if self.peek() == Some('=') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::EqualEqual, "==")
                } else {
                    self.make_token_with_char(TokenKind::Equal, ch)
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::BangEqual, "!=")
                } else {
                    self.make_token_with_char(TokenKind::Bang, ch)
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::LessEqual, "<=")
                } else {
                    self.make_token_with_char(TokenKind::Less, ch)
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    self.make_token_with_lexeme(TokenKind::GreaterEqual, ">=")
                } else {
                    self.make_token_with_char(TokenKind::Greater, ch)
                }
            }
            '0'..='9' => self.lex_number(ch),
            '"' => self.lex_string(),
            '\'' => self.lex_char(),
            'a'..='z' | 'A'..='Z' | '_' => self.lex_identifier_or_keyword(ch),
            _ => self.error_token(ch),
        }
    }

    fn make_token_with_char(&self, kind: TokenKind, ch: char) -> Token {
        Token {
            kind,
            span: self.span(),
            lexeme: ch.to_string(),
        }
    }

    fn make_token_with_lexeme(&self, kind: TokenKind, lexeme: &str) -> Token {
        Token {
            kind,
            span: self.span(),
            lexeme: lexeme.to_string(),
        }
    }

    fn error_token(&self, ch: char) -> Token {
        Token {
            kind: TokenKind::Error,
            span: self.span(),
            lexeme: ch.to_string(),
        }
    }

    fn lex_number(&mut self, first: char) -> Token {
        let mut lexeme = first.to_string();
        let mut has_dot = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                lexeme.push(self.bump().unwrap());
            } else if ch == '.' && !has_dot {
                // 简单浮点支持，需注意范围
                has_dot = true;
                lexeme.push(self.bump().unwrap());
            } else if ch == 'e' || ch == 'E' {
                lexeme.push(self.bump().unwrap());
                if let Some(next) = self.peek() {
                    if next == '+' || next == '-' {
                        lexeme.push(self.bump().unwrap());
                    }
                }
            } else {
                break;
            }
        }
        let kind = if has_dot || lexeme.contains('e') || lexeme.contains('E') {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };
        Token {
            kind,
            span: self.span(),
            lexeme,
        }
    }

    fn lex_string(&mut self) -> Token {
        let mut lexeme = String::new();
        while let Some(ch) = self.bump() {
            if ch == '"' {
                return Token {
                    kind: TokenKind::StringLiteral,
                    span: self.span(),
                    lexeme,
                };
            }
            lexeme.push(ch);
        }
        // 未闭合的字符串
        Token {
            kind: TokenKind::Error,
            span: self.span(),
            lexeme,
        }
    }

    fn lex_char(&mut self) -> Token {
        let ch = match self.bump() {
            Some(c) => c,
            None => {
                return Token {
                    kind: TokenKind::Error,
                    span: self.span(),
                    lexeme: String::new(),
                };
            }
        };
        if self.peek() == Some('\'') {
            self.bump();
            Token {
                kind: TokenKind::CharLiteral,
                span: self.span(),
                lexeme: ch.to_string(),
            }
        } else {
            Token {
                kind: TokenKind::Error,
                span: self.span(),
                lexeme: ch.to_string(),
            }
        }
    }

    fn lex_identifier_or_keyword(&mut self, first: char) -> Token {
        let mut lexeme = first.to_string();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                lexeme.push(self.bump().unwrap());
            } else {
                break;
            }
        }
        let kind = is_keyword(lexeme.as_str());
        Token {
            kind,
            span: self.span(),
            lexeme,
        }
    }
}

/// we can now use 'for token in lexer' to iterate over tokens, and it will stop at TokenEOF.
impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token.kind == TokenKind::TokenEOF {
            None // 常规做法，迭代器返回 None 表示结束
        } else {
            Some(token)
        }
    }
}

