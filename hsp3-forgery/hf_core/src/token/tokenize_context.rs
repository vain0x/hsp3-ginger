use super::*;
use std::cmp::min;
use std::rc::Rc;

/// 字句解析の状態を持つもの。
pub(crate) struct TokenizeContext {
    source: TokenSource,

    source_code: Rc<SourceCode>,

    cursor: TextCursor,

    current_index: usize,

    /// 直前のコミット位置
    last_index: usize,

    /// 次のトークンの前方に改行や空白があるか？
    leading: bool,

    tokens: Vec<TokenData>,
}

impl TokenizeContext {
    pub(crate) fn new(source: TokenSource, source_code: Rc<SourceCode>) -> Self {
        TokenizeContext {
            source,
            source_code,
            cursor: TextCursor::default(),
            current_index: 0,
            last_index: 0,
            leading: true,
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

    pub(crate) fn nth(&self, index: usize) -> char {
        self.source_code[min(self.current_index + index, self.source_code.len())..]
            .chars()
            .next()
            .unwrap_or('\0')
    }

    pub(crate) fn next(&self) -> char {
        self.nth(0)
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

    pub(crate) fn is_followed_by(&self, text: &str) -> bool {
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

    fn push_token(&mut self, token_data: TokenData) {
        self.tokens.push(token_data);
    }

    /// コミットする。前回のコミット位置から現在位置までの間を1個のトークンとみなす。
    pub(crate) fn commit(&mut self, token: Token) {
        let text = self.current_text().to_string();

        let start = self.cursor.current();
        self.cursor.advance(&text);
        let end = self.cursor.current();
        let location = Location {
            source: self.source.clone(),
            range: Range { start, end },
        };

        self.push_token(TokenData::new(token, text, location));
        self.last_index = self.current_index;

        self.assert_invariants();
    }

    pub(crate) fn finish(mut self) -> Vec<TokenData> {
        assert_eq!(self.current_index, self.last_index);
        assert_eq!(self.current_index, self.source_code.len());

        self.commit(Token::Eof);

        assert!(self
            .tokens
            .last()
            .map_or(false, |t| t.token() == Token::Eof));

        self.tokens
    }
}
