use super::*;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Token {
    Eof,
    Eol,
    Space,
    Comment,
    /// 文字列やコメントの外では解釈できない文字
    Other,
    /// "0b"
    ZeroB,
    /// "0x"
    ZeroX,
    /// 0-9 の並び
    Digit,
    /// 0-1 の並び
    Binary,
    /// 0-9/a-f/A-F の並び
    Hex,
    /// 文字・文字列リテラル内部のエスケープされない文字の並び
    StrVerbatim,
    /// 文字・文字列リテラル内部のエスケープ1個
    StrEscape,
    /// キーワードではない識別子
    Ident,

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
    /// `{"`
    LeftQuote,
    /// `"}`
    RightQuote,
    AndAnd,
    And,
    AtSign,
    Backslash,
    BangEqual,
    Bang,
    Colon,
    Comma,
    Dollar,
    Dot,
    DoubleQuote,
    EqualEqual,
    Equal,
    /// 行頭にある `#`
    /// 行頭にない場合は Other になる。
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

    pub(crate) location: Location,

    /// 前にスペースや改行がある？
    pub(crate) leading: bool,

    /// 後ろにスペースや改行がある？
    pub(crate) trailing: bool,
}

impl TokenData {
    pub(crate) fn new(token: Token, text: String, location: Location) -> Self {
        TokenData {
            token,
            text,
            location,
            leading: false,
            trailing: false,
        }
    }

    pub(crate) fn new_missing(hint_location: Location) -> Self {
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
        write!(f, "{:?}({:?})", self.token(), self.text())
    }
}
