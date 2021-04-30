use super::TokenKind;
use crate::{source::ALoc, utils::rc_str::RcStr};
use std::fmt;

/// 字句データ
#[derive(Clone)]
#[must_use]
pub(crate) struct TokenData {
    pub(crate) kind: TokenKind,
    pub(crate) text: RcStr,
    pub(crate) loc: ALoc,
}

impl fmt::Debug for TokenData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TokenKind::Blank => write!(f, "Blank({})", self.text.len()),
            TokenKind::Newlines => write!(f, "Newlines({:?})", self.text),
            TokenKind::Comment
            | TokenKind::Number
            | TokenKind::Char
            | TokenKind::Str
            | TokenKind::Ident
            | TokenKind::Bad => fmt::Debug::fmt(&self.text, f),
            _ => write!(f, "{:?}", self.kind),
        }
    }
}
