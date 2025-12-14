use super::p_token::PToken;
use crate::token::TokenKind;

/// Parsing context. 構文解析の文脈
pub(crate) struct Px {
    /// トークン列を逆順に並べたもの
    tokens: Vec<PToken>,
    /// スキップしたトークン
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

    /// 現在位置から offset (>= 0) 番目のトークン
    pub(crate) fn nth_token(&self, offset: usize) -> &PToken {
        assert!(offset < self.tokens.len());

        self.tokens.get(self.tokens.len() - offset - 1).unwrap()
    }

    /// 現在位置から offset (>= 0) 番目のトークンの種類
    pub(crate) fn nth(&self, offset: usize) -> TokenKind {
        if offset < self.tokens.len() {
            self.tokens[self.tokens.len() - offset - 1].kind()
        } else {
            TokenKind::Eof
        }
    }

    /// 次のトークン
    pub(crate) fn next_token(&self) -> &PToken {
        self.nth_token(0)
    }

    /// 次のトークンの種類
    pub(crate) fn next(&self) -> TokenKind {
        self.nth(0)
    }

    /// トークンを取り出す
    pub(crate) fn bump(&mut self) -> PToken {
        assert!(!self.tokens.is_empty());

        self.tokens.pop().unwrap()
    }

    /// 次のトークンが指定した種類なら取り出す
    pub(crate) fn eat(&mut self, kind: TokenKind) -> Option<PToken> {
        if self.next() == kind {
            Some(self.bump())
        } else {
            None
        }
    }

    /// 次のトークンをスキップする
    pub(crate) fn skip(&mut self) {
        let token = self.bump();
        assert_ne!(token.kind(), TokenKind::Eof);

        self.skipped.push(token);
    }

    /// 構文解析工程を終了する
    pub(crate) fn finish(mut self) -> (Vec<PToken>, PToken) {
        assert_eq!(self.tokens.len(), 1);
        assert_eq!(self.next(), TokenKind::Eof);

        let eof = self.bump();
        (self.skipped, eof)
    }
}
