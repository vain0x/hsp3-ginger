use super::*;
use std::rc::Rc;

/// 構文解析の状態を持つもの。
pub(crate) struct ParseContext {
    current: GreenNode,
    stack: Vec<GreenNode>,
    tokens: Vec<FatToken>,
}

impl ParseContext {
    pub(crate) fn new(mut tokens: Vec<FatToken>) -> Self {
        tokens.reverse();

        ParseContext {
            current: GreenNode::new(),
            stack: vec![],
            tokens,
        }
    }

    pub(crate) fn current_index(&self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.next() == Token::Eof
    }

    fn nth_data(&self, offset: usize) -> Option<&FatToken> {
        assert!(offset < self.tokens.len());

        self.tokens.get(self.tokens.len() - offset - 1)
    }

    pub(crate) fn next(&self) -> Token {
        self.nth_data(0).map_or(Token::Eof, |token| token.token())
    }

    pub(crate) fn next_data(&self) -> &FatToken {
        self.nth_data(0).unwrap()
    }

    pub(crate) fn nth(&self, offset: usize) -> Token {
        self.nth_data(offset).map_or(Token::Eof, |t| t.token())
    }

    pub(crate) fn bump(&mut self) {
        assert!(!self.tokens.is_empty());

        let token = self.tokens.pop().unwrap();
        self.current.push_fat_token(token);
    }

    pub(crate) fn eat(&mut self, token: Token) -> bool {
        if self.next() == token {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_ident(&mut self, text: &str) -> bool {
        if self.next() == Token::Ident && self.next_data().text() == text {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn start_node(&mut self) {
        // 現在のノードをスタックに積み、新しいノードを current にする。
        let mut node = GreenNode::new();
        std::mem::swap(&mut node, &mut self.current);
        self.stack.push(node);
    }

    pub(crate) fn restart_node(&mut self) {
        // 新しいノードを current にして、現在のノードの最後のノード以降の子要素をそれの子要素として移植する。
        // 例えば `f(x + y)` をパースするとき、`+` の直前において current = `f(x ` である。
        // `+` を読んだ段階で、`x` を本来なら `+` の子要素にしなければならなかったことになるので、
        // この関数を使って `f(` → `x +` という構造に変える。

        let mut node = GreenNode::new();
        std::mem::swap(&mut node, &mut self.current);
        self.current.drain_last_node_from(&mut node);
        self.stack.push(node);
    }

    pub(crate) fn end_node(&mut self, kind: NodeKind) {
        // 現在のノードを完了させてスタックトップの末子とし、
        // スタックトップを current に戻す。
        let mut node = self
            .stack
            .pop()
            .expect("start_node/end_node が対応していません。");

        std::mem::swap(&mut node, &mut self.current);

        node.set_kind(kind);
        self.current.push_node(node);
    }

    pub(crate) fn finish(mut self) -> Rc<SyntaxRoot> {
        assert_eq!(self.tokens.len(), 1);
        assert_eq!(self.next(), Token::Eof);
        self.bump();

        assert_eq!(self.stack.len(), 0);
        self.current.set_kind(NodeKind::Root);

        SyntaxRoot::new(self.current)
    }
}
