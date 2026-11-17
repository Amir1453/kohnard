use logos::Logos;

use std::fmt;
use std::num::ParseIntError;

use crate::frontend::Symbol;

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexicalError {
    InvalidInteger(ParseIntError),
    #[default]
    InvalidToken,
}

impl From<ParseIntError> for LexicalError {
    fn from(err: ParseIntError) -> Self {
        LexicalError::InvalidInteger(err)
    }
}

impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexicalError::InvalidInteger(err) => write!(f, "Invalid integer: {}", err),
            LexicalError::InvalidToken => write!(f, "Invalid token encountered"),
        }
    }
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+", skip r"//.*\n?", skip r"extern.*\n?", error = LexicalError)]
pub enum Token {
    #[token("def")]
    KeywordDef,
    #[token("auto")]
    KeywordAuto,
    #[token("fn")]
    KeywordFn,
    #[token("var")]
    KeywordVar,
    #[token("let")]
    KeywordLet,
    #[token("mut")]
    KeywordMut,
    #[token("int")]
    KeywordInt,
    #[token("bool")]
    KeywordBool,
    #[token("void")]
    KeywordVoid,
    #[token("return")]
    KeywordReturn,
    #[token("if")]
    KeywordIf,
    #[token("else")]
    KeywordElse,
    #[token("while")]
    KeywordWhile,
    #[token("break")]
    KeywordBreak,
    #[token("continue")]
    KeywordContinue,
    #[token("true")]
    KeywordTrue,
    #[token("false")]
    KeywordFalse,

    #[regex("[_a-zA-Z][_0-9a-zA-Z]*", |lex| Symbol::intern(lex.slice()))]
    Identifier(Symbol),
    #[regex("-?[0-9]*", |lex| lex.slice().parse())]
    Number(i64),

    #[token(",")]
    Comma,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LCurlyParen,
    #[token("}")]
    RCurlyParen,
    #[token("=")]
    Assign,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("->")]
    Arrow,

    #[token("+")]
    OpPlus,
    #[token("-")]
    OpMinus,
    #[token("*")]
    OpStar,
    #[token("/")]
    OpSlash,
    #[token("%")]
    OpMod,

    #[token("|")]
    OpPipe,
    #[token("^")]
    OpCaret,
    #[token("&")]
    OpAmpersand,
    #[token("~")]
    OpTilde,
    #[token("<<")]
    OpLShift,
    #[token(">>")]
    OpRShift,

    #[token("==")]
    OpEq,
    #[token("!=")]
    OpNEq,
    #[token("<")]
    OpL,
    #[token("<=")]
    OpLTE,
    #[token(">")]
    OpG,
    #[token(">=")]
    OpGTE,
    #[token("&&")]
    OpLAnd,
    #[token("||")]
    OpLOr,
    #[token("!")]
    OpLNot,

    Error(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
