use super::*;
use std::rc::Rc;

pub(crate) struct ParseContext {
    current: GreenNode,
    stack: Vec<GreenNode>,
    tokens: Vec<TokenData>,
}

impl ParseContext {
    pub(crate) fn new(mut tokens: Vec<TokenData>) -> Self {
        tokens.reverse();

        ParseContext {
            current: GreenNode::new_root(),
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

    fn nth(&self, offset: usize) -> Option<&TokenData> {
        assert!(offset < self.tokens.len());

        self.tokens.get(self.tokens.len() - offset - 1)
    }

    pub(crate) fn next(&self) -> Token {
        self.nth(0).map_or(Token::Eof, |token| token.token())
    }

    pub(crate) fn nth_data(&self, offset: usize) -> Option<&TokenData> {
        self.nth(offset)
    }

    pub(crate) fn next_data(&self) -> &TokenData {
        self.nth(0).unwrap()
    }

    pub(crate) fn bump(&mut self) {
        assert!(!self.tokens.is_empty());

        let token = self.tokens.pop().unwrap();
        self.current.push_token(token);
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
        let mut node = GreenNode::new_dummy();
        std::mem::swap(&mut node, &mut self.current);
        self.stack.push(node);
    }

    pub(crate) fn restart_node(&mut self) {
        // 新しいノードを current にして、現在のノードの最後のノード以降の子要素をそれの子要素として移植する。
        // 例えば `f(x + y)` をパースするとき、`+` の直前において current = `f(x ` である。
        // `+` を読んだ段階で、`x` を本来なら `+` の子要素にしなければならなかったことになるので、
        // この関数を使って `f(` → `x +` という構造に変える。

        let mut node = GreenNode::new_dummy();
        std::mem::swap(&mut node, &mut self.current);

        // 最後のノードの位置を計算する。
        let mut i = node.children.len();
        while i >= 1 {
            i -= 1;
            match node.children[i] {
                GreenElement::Node(..) => break,
                GreenElement::Token(..) => continue,
            }
        }

        self.current.children.extend(node.children.drain(i..));
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

        assert_eq!(self.stack.len(), 0);
        assert_eq!(self.current.kind, NodeKind::Root);

        self.bump();

        SyntaxRoot::new(self.current)
    }
}
