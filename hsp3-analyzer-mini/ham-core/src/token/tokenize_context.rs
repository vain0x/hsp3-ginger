use super::{TokenData, TokenKind};
use crate::{
    analysis::{ADoc, ALoc, APos},
    utils::rc_str::RcStr,
};

/// Tokenization context. 字句解析の文脈
pub(crate) struct TokenizeContext {
    doc: ADoc,
    source_code: RcStr,
    current_index: usize,
    last_index: usize,
    last_position: APos,
    tokens: Vec<TokenData>,
}

impl TokenizeContext {
    pub(crate) fn new(doc: ADoc, source_code: RcStr) -> Self {
        TokenizeContext {
            doc,
            source_code,
            current_index: 0,
            last_index: 0,
            last_position: APos::default(),
            tokens: vec![],
        }
    }

    pub(crate) fn assert_invariants(&self) {
        assert!(self.last_index <= self.current_index);
        assert!(self.current_index <= self.source_code.len());
        assert!(self.source_code.is_char_boundary(self.current_index));
    }

    pub(crate) fn nth(&self, index: usize) -> char {
        self.source_code[(self.current_index + index).min(self.source_code.len())..]
            .chars()
            .next()
            .unwrap_or('\0')
    }

    pub(crate) fn nth_byte(&self, index: usize) -> u8 {
        self.source_code
            .as_bytes()
            .get((self.current_index + index).min(self.source_code.len()))
            .copied()
            .unwrap_or(b'\0')
    }

    pub(crate) fn next(&self) -> char {
        self.nth(0)
    }

    pub(crate) fn find(&self, pattern: &str) -> Option<usize> {
        self.source_code[self.current_index..].find(pattern)
    }

    pub(crate) fn current_text(&self) -> &str {
        &self.source_code[self.last_index..self.current_index]
    }

    pub(crate) fn bump_many(&mut self, len: usize) {
        assert!(self.current_index + len <= self.source_code.len());
        assert!(self.source_code.is_char_boundary(self.current_index + len));

        self.current_index += len;

        self.assert_invariants();
    }

    pub(crate) fn bump_all(&mut self) {
        self.current_index = self.source_code.len();

        self.assert_invariants();
    }

    pub(crate) fn bump(&mut self) {
        self.bump_many(self.next().len_utf8());
    }

    pub(crate) fn is_followed_by(&self, text: &str) -> bool {
        self.source_code[self.current_index..].starts_with(text)
    }

    pub(crate) fn eat(&mut self, text: &str) -> bool {
        if self.is_followed_by(text) {
            self.bump_many(text.len());
            true
        } else {
            false
        }
    }

    fn push_token(&mut self, token_data: TokenData) {
        self.tokens.push(token_data);
    }

    pub(crate) fn commit(&mut self, kind: TokenKind) {
        let text = self.source_code.slice(self.last_index, self.current_index);

        let current_position = self.last_position.add(APos::from_str(text.as_str()));
        let loc = ALoc::new3(self.doc, self.last_position, current_position);
        let token = TokenData { kind, text, loc };

        self.push_token(token);

        self.last_index = self.current_index;
        self.last_position = current_position;

        self.assert_invariants();
    }

    pub(crate) fn finish(mut self) -> Vec<TokenData> {
        assert_eq!(self.current_index, self.last_index);
        assert_eq!(self.current_index, self.source_code.len());

        self.commit(TokenKind::Eof);

        assert!(self
            .tokens
            .last()
            .map_or(false, |t| t.kind == TokenKind::Eof));

        self.tokens
    }
}
