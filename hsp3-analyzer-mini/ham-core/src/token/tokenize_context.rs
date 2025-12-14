use super::*;

/// Tokenization context. 字句解析の文脈
pub(crate) struct TokenizeContext {
    doc: DocId,
    source_code: RcStr,
    /// 解析の現在位置
    current_index: usize,
    /// 直前の確定位置
    last_index: usize,
    last_position: Pos,
    tokens: Vec<TokenData>,
}

impl TokenizeContext {
    pub(crate) fn new(doc: DocId, source_code: RcStr) -> Self {
        TokenizeContext {
            doc,
            source_code,
            current_index: 0,
            last_index: 0,
            last_position: Pos::default(),
            tokens: vec![],
        }
    }

    /// 不変条件を検証する
    pub(crate) fn assert_invariants(&self) {
        assert!(self.last_index <= self.current_index);
        assert!(self.current_index <= self.source_code.len());
        assert!(self.source_code.is_char_boundary(self.current_index));
    }

    /// index (>= 0) の位置にある文字 (なければヌル文字)
    pub(crate) fn nth(&self, index: usize) -> char {
        self.source_code[(self.current_index + index).min(self.source_code.len())..]
            .chars()
            .next()
            .unwrap_or('\0')
    }

    /// index (>= 0) の位置にある1バイト (なければ 0)
    pub(crate) fn nth_byte(&self, index: usize) -> u8 {
        self.source_code
            .as_bytes()
            .get((self.current_index + index).min(self.source_code.len()))
            .copied()
            .unwrap_or(b'\0')
    }

    /// 次の文字
    pub(crate) fn next(&self) -> char {
        self.nth(0)
    }

    pub(crate) fn find(&self, pattern: &str) -> Option<usize> {
        self.source_code[self.current_index..].find(pattern)
    }

    /// 未確定範囲のテキスト
    pub(crate) fn current_text(&self) -> &str {
        &self.source_code[self.last_index..self.current_index]
    }

    /// 一定バイト数だけ前進する
    pub(crate) fn bump_many(&mut self, len: usize) {
        assert!(self.current_index + len <= self.source_code.len());
        assert!(self.source_code.is_char_boundary(self.current_index + len));

        self.current_index += len;

        self.assert_invariants();
    }

    /// ファイル末尾まで前進する
    pub(crate) fn bump_all(&mut self) {
        self.current_index = self.source_code.len();

        self.assert_invariants();
    }

    /// 1文字分前進する
    pub(crate) fn bump(&mut self) {
        self.bump_many(self.next().len_utf8());
    }

    pub(crate) fn is_followed_by(&self, text: &str) -> bool {
        self.source_code[self.current_index..].starts_with(text)
    }

    /// 指定した文字列があれば読み取りを進める
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

    /// トークンの解析を確定する
    ///
    /// 直前の確定位置 (last_index) から現在位置 (current_index) までを1つのトークンとして確定する
    pub(crate) fn commit(&mut self, kind: TokenKind) {
        let text = self.source_code.slice(self.last_index, self.current_index);

        let current_position = self.last_position + Pos::from(text.as_str());
        let loc = Loc::new3(self.doc, self.last_position, current_position);
        let token = TokenData { kind, text, loc };

        self.push_token(token);

        self.last_index = self.current_index;
        self.last_position = current_position;

        self.assert_invariants();
    }

    /// 字句解析工程を終了する
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
