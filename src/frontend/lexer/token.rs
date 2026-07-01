use crate::common::span::Span;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    LeftParen,  // (
    RightParen, // )
    LeftBrace,  // {
    RightBrace, // }
    Comma,      // ,
    Dot,        // .
    Minus,      //-
    Plus,       //=
    Caret,      // ^
    Semicolon,  // ;
    Slash,      // /
    Star,       // *
    Backslash,  // (\)
    Modulo,     // %
    Colon,      // :

    // One or two character tokens.
    RightBracket, // ]
    LeftBracket,  // [
    Bang,         // !
    BangEqual,    // !=
    Equal,        // =
    EqualEqual,   //==
    Greater,      // >
    GreaterEqual, //>=
    Less,         //<
    LessEqual,    //<=
    Arrow,        // ->
    MinusMinus,   //--
    PlusPlus,     //++

    // Literals.
    Identifier,
    CharLiteral,
    StringLiteral,
    IntLiteral,
    FloatLiteral,

    // Keywords.
    // logic operators
    And,
    Or,
    True,
    False,
    // branch
    If,
    Else,
    Elif,
    // declare
    Decl,
    Def,
    Var,
    // return
    Return,
    // none
    None,
    // refernce
    Ref,
    // class
    Class,
    Super,
    Sself,
    New,
    // loop
    While,
    // control flow
    Break,
    Continue,
    // type
    Int,
    Float,
    Bool,
    Char,
    Str,
    // list
    List,

    TokenEOF,
    Error,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

