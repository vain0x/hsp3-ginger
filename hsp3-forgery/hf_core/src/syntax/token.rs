use super::*;
use std::fmt::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Token {
    Eof,
    Eol,
    Space,
    Digit,
    StrVerbatim,
    StrEscape,
    Ident,
    Other,

    // キーワード
    Break,
    Cnt,
    Continue,
    Else,
    End,
    Gosub,
    Goto,
    If,
    Loop,
    Refdval,
    Refstr,
    Repeat,
    Return,
    Stat,
    Stop,
    Strsize,

    // 約物
    LeftParen,
    RightParen,
    LeftAngle,
    RightAngle,
    LeftBrace,
    RightBrace,
    LeftQuote,
    RightQuote,
    AndAnd,
    And,
    AtSign,
    BangEqual,
    Bang,
    Backslash,
    Colon,
    Comma,
    Dollar,
    Dot,
    DoubleQuote,
    EqualEqual,
    Equal,
    Hash,
    Hat,
    LeftShift,
    LeftEqual,
    Minus,
    Percent,
    Pipe,
    PipePipe,
    Plus,
    RightEqual,
    RightShift,
    SingleQuote,
    Slash,
    Star,
}

#[derive(Clone)]
pub(crate) struct TokenData {
    token: Token,
    text: String,

    pub(crate) location: SourceLocation,

    /// 行頭？
    pub(crate) line_head: bool,

    /// 前にスペースや改行がある？
    pub(crate) leading: bool,

    /// 後ろにスペースや改行がある？
    pub(crate) trailing: bool,
}

impl TokenData {
    pub(crate) fn new(token: Token, text: String, location: SourceLocation) -> Self {
        TokenData {
            token,
            text,
            location,
            line_head: false,
            leading: false,
            trailing: false,
        }
    }

    pub(crate) fn new_missing(hint_location: SourceLocation) -> Self {
        TokenData::new(Token::Other, "???".to_string(), hint_location)
    }

    pub(crate) fn token(&self) -> Token {
        self.token
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }
}

impl fmt::Debug for TokenData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.text())
    }
}
