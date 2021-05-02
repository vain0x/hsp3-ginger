use super::p_token::PToken;
use crate::token::TokenKind;

/// Parsing context. 構文解析の文脈
pub(crate) struct Px {
    /// トークン列を逆順に並べたもの。
    tokens: Vec<PToken>,
    /// 無視したトークン
    skipped: Vec<PToken>,
}

impl Px {
    pub(crate) fn new(mut tokens: Vec<PToken>) -> Self {
        tokens.reverse();

        Px {
            tokens,
            skipped: vec![],
        }
    }

    pub(crate) fn nth_token(&self, offset: usize) -> &PToken {
        assert!(offset < self.tokens.len());

        self.tokens.get(self.tokens.len() - offset - 1).unwrap()
    }

    pub(crate) fn nth(&self, offset: usize) -> TokenKind {
        self.nth_token(offset).kind()
    }

    pub(crate) fn next_token(&self) -> &PToken {
        self.nth_token(0)
    }

    pub(crate) fn next(&self) -> TokenKind {
        self.nth(0)
    }

    pub(crate) fn bump(&mut self) -> PToken {
        assert!(!self.tokens.is_empty());

        self.tokens.pop().unwrap()
    }

    pub(crate) fn eat(&mut self, kind: TokenKind) -> Option<PToken> {
        if self.next() == kind {
            Some(self.bump())
        } else {
            None
        }
    }

    pub(crate) fn skip(&mut self) {
        let token = self.bump();
        assert_ne!(token.kind(), TokenKind::Eof);

        self.skipped.push(token);
    }

    pub(crate) fn finish(mut self) -> (Vec<PToken>, PToken) {
        assert_eq!(self.tokens.len(), 1);
        assert_eq!(self.next(), TokenKind::Eof);

        let eof = self.bump();
        (self.skipped, eof)
    }
}
