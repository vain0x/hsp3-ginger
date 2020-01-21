//! 字句解析の状態管理

use super::*;
use std::rc::Rc;

pub(crate) struct TokenizeContext {
    source_code: Rc<String>,

    current_index: usize,

    /// 直前のコミット位置
    last_index: usize,

    /// 次のトークンの前方にあるトリビア
    leading: Vec<Trivia>,

    tokens: Vec<FatToken>,
}

impl TokenizeContext {
    pub(crate) fn new(source_code: Rc<String>) -> Self {
        TokenizeContext {
            source_code,
            current_index: 0,
            last_index: 0,
            leading: vec![],
            tokens: vec![],
        }
    }

    pub(crate) fn assert_invariants(&self) {
        assert!(self.last_index <= self.current_index);
        assert!(self.current_index <= self.source_code.len());
    }

    pub(crate) fn current_index(&self) -> usize {
        self.current_index
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.current_index >= self.source_code.len()
    }

    pub(crate) fn next(&self) -> char {
        self.source_code[self.current_index..]
            .chars()
            .next()
            .unwrap_or('\0')
    }

    pub(crate) fn current_text(&self) -> &str {
        &self.source_code[self.last_index..self.current_index]
    }

    fn bump_many(&mut self, len: usize) {
        assert!(self.current_index + len <= self.source_code.len());

        self.current_index += len;

        self.assert_invariants();
    }

    /// 次の1文字を読み進める。
    pub(crate) fn bump(&mut self) {
        self.bump_many(self.next().len_utf8());
    }

    fn is_followed_by(&self, text: &str) -> bool {
        self.source_code[self.current_index..].starts_with(text)
    }

    /// 現在位置に続いて指定した文字列が出現するなら、それを読み進めて true を返す。
    /// そうでなければ、何もせず false を返す。
    pub(crate) fn eat(&mut self, text: &str) -> bool {
        if self.is_followed_by(text) {
            self.bump_many(text.len());
            true
        } else {
            false
        }
    }

    fn push_token(&mut self, mut token_data: TokenData) {
        let token = token_data.token();

        if token.is_trailing_trivia() && !self.tokens.is_empty() && self.leading.is_empty() {
            self.tokens
                .last_mut()
                .unwrap()
                .push_trailing(Trivia::from(token_data));
        } else if token.is_leading_trivia() {
            self.leading.push(Trivia::from(token_data));
        } else {
            let mut fat_token = FatToken::from(token_data);

            for t in self.leading.drain(..) {
                fat_token.push_leading(t);
            }

            self.tokens.push(fat_token);
        }
    }

    /// コミットする。前回のコミット位置から現在位置までの間を1個の字句とみなす。
    pub(crate) fn commit(&mut self, token: Token) {
        let text = self.current_text().to_string();

        self.push_token(TokenData::new(token, text));
        self.last_index = self.current_index;

        self.assert_invariants();
    }

    pub(crate) fn finish(mut self) -> Box<[FatToken]> {
        assert_eq!(self.current_index, self.last_index);
        assert_eq!(self.current_index, self.source_code.len());

        self.push_token(TokenData::new(Token::Eof, "".to_string()));

        assert!(self.leading.is_empty());
        assert!(self
            .tokens
            .last()
            .map_or(false, |t| t.token() == Token::Eof));

        self.tokens.into_boxed_slice()
    }
}
