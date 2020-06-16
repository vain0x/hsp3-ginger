use super::TokenKind;
use crate::{analysis::ALoc, utils::rc_str::RcStr};
use std::fmt;

/// 字句データ
#[derive(Clone)]
pub(crate) struct TokenData {
    pub(crate) kind: TokenKind,
    pub(crate) text: RcStr,
    pub(crate) loc: ALoc,
}

impl fmt::Debug for TokenData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TokenKind::Space => write!(f, "Space({})", self.text.len()),
            TokenKind::Comment
            | TokenKind::Number
            | TokenKind::Char
            | TokenKind::Str
            | TokenKind::Ident
            | TokenKind::Other => fmt::Debug::fmt(&self.text, f),
            _ => write!(f, "{:?}", self.kind),
        }
    }
}
