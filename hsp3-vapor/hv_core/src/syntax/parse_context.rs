//! 構文解析の状態管理

use super::*;
use std::rc::Rc;

type TokenIndex = usize;

type TokenList = Rc<[FatToken]>;

pub(crate) struct ParseContext {
    tokens: TokenList,
    index: TokenIndex,
}

impl ParseContext {
    pub(crate) fn new(tokens: TokenList) -> Self {
        ParseContext { tokens, index: 0 }
    }

    pub(crate) fn assert_invariants(&self) {
        assert!(self.index <= self.tokens.len());
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.next() == Token::Eof
    }

    /// 次の字句との間に改行があるか？
    pub(crate) fn at_eol(&self) -> bool {
        if self.index >= self.tokens.len() {
            return true;
        }

        self.tokens[self.index].contains_eol()
    }

    fn nth(&self, offset: usize) -> Option<&FatToken> {
        self.tokens.get(self.index + offset)
    }

    pub(crate) fn next(&self) -> Token {
        self.nth(0).map_or(Token::Eof, |token| token.token())
    }

    pub(crate) fn snapshot(&self, node: &mut NodeData) -> (usize, usize) {
        (self.index, node.snapshot())
    }

    pub(crate) fn rollback(&mut self, node: &mut NodeData, snapshot: (usize, usize)) {
        assert!(0 <= self.tokens.len());

        let (index, node_snapshot) = snapshot;

        self.index = index;
        node.rollback(node_snapshot);
    }

    /// 次の字句を読み進める。
    pub(crate) fn bump(&mut self, node: &mut NodeData) {
        assert!(self.index + 1 <= self.tokens.len());

        let token = &self.tokens[self.index];

        node.push_token(token.clone());

        self.index += 1;
        self.assert_invariants();
    }

    /// 次の字句が指定された種類なら読み進める。
    pub(crate) fn eat(&mut self, node: &mut NodeData, token: Token) -> bool {
        if self.next() == token {
            self.bump(node);
            true
        } else {
            false
        }
    }

    pub(crate) fn finish(mut self, root: &mut NodeData) {
        assert_eq!(self.index, self.tokens.len() - 1);
        assert_eq!(root.node(), Node::Root);

        self.bump(root);
    }
}
