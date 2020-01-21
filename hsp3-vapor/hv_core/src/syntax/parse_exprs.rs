use super::parse_context::ParseContext;
use super::*;

impl Token {
    pub(crate) fn is_atom_first(self) -> bool {
        self == Token::Number || self == Token::Ident || self == Token::LeftParen
    }

    /// 次のトークンが式の FIRST 集合に入っているか？
    ///
    /// 次のトークンから始まるような式の構文があるなら true。
    pub(crate) fn is_expr_first(self) -> bool {
        self.is_atom_first()
    }
}

/// 原子式をパースする。
pub(crate) fn parse_atom(p: &mut ParseContext) -> Option<NodeData> {
    match p.next() {
        Token::Number => {
            let mut node = NodeData::new();
            p.bump(&mut node);
            Some(node.set_node(Node::NumberLiteral))
        }
        Token::Ident => {
            let mut node = NodeData::new();
            p.bump(&mut node);
            Some(node.set_node(Node::Name))
        }
        Token::LeftParen => {
            let mut node = NodeData::new();
            p.bump(&mut node);

            if let Some(body) = parse_expr(p) {
                node.push_node(body);
            } else {
                node.push_error(ParseError::ExpectedExpr);
            }

            if p.next() == Token::RightParen {
                p.bump(&mut node);
            } else {
                node.push_error(ParseError::ExpectedRightParen);
            }

            Some(node.set_node(Node::Group))
        }
        _ => {
            debug_assert!(!p.next().is_atom_first());
            None
        }
    }
}

/// 関数呼び出しをパースする。
pub(crate) fn parse_call(p: &mut ParseContext) -> Option<NodeData> {
    let mut callee = parse_atom(p)?;

    while p.next() == Token::LeftParen {
        // FIXME: callee が識別子でなければ構文エラー
        let mut node = NodeData::new_before(callee);
        p.bump(&mut node);

        while let Some(arg) = parse_expr(p) {
            let arg = NodeData::new_before(arg);
            node.push_node(arg.set_node(Node::Argument));

            p.eat(&mut node, Token::Comma);
        }

        if !p.eat(&mut node, Token::RightParen) {
            node.push_error(ParseError::ExpectedRightParen);
        }

        callee = node.set_node(Node::Call);
    }

    Some(callee)
}

/// `K {}` 形式のデータ構築以外の式をパースする。
pub(crate) fn parse_cond(p: &mut ParseContext) -> Option<NodeData> {
    parse_call(p)
}

pub(crate) fn parse_expr(p: &mut ParseContext) -> Option<NodeData> {
    parse_call(p)
}
