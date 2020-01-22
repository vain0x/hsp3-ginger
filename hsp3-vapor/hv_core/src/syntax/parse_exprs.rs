use super::parse_context::ParseContext;
use super::*;

impl Token {
    pub(crate) fn is_atom_expr_first(self) -> bool {
        self == Token::Number || self == Token::Ident || self == Token::LeftParen
    }

    pub(crate) fn is_element_first(self) -> bool {
        self == Token::Ident
    }

    pub(crate) fn is_arg_first(self) -> bool {
        self.is_expr_first() || self == Token::Comma
    }

    /// 次のトークンが式の FIRST 集合に入っているか？
    ///
    /// 次のトークンから始まるような式の構文があるなら true。
    pub(crate) fn is_expr_first(self) -> bool {
        self.is_atom_expr_first()
    }
}

pub(crate) fn parse_args(p: &mut ParseContext, node: &mut NodeData) {
    while p.next().is_arg_first() {
        let mut arg = NodeData::new();

        if p.next().is_expr_first() {
            arg.push_node(parse_expr(p));
        }

        if p.next() == Token::Comma {
            p.bump(&mut arg);
        }

        node.push_node(arg.set_node(Node::Arg));
    }
}

fn parse_number(p: &mut ParseContext) -> NodeData {
    assert_eq!(p.next(), Token::Number);

    let mut node = NodeData::new();
    p.bump(&mut node);
    node.set_node(Node::NumberLiteral)
}

pub(crate) fn parse_name(p: &mut ParseContext) -> NodeData {
    assert_eq!(p.next(), Token::Ident);

    let mut node = NodeData::new();
    p.eat(&mut node, Token::Ident);
    node.set_node(Node::Name)
}

fn parse_group(p: &mut ParseContext) -> NodeData {
    assert_eq!(p.next(), Token::LeftParen);

    let mut node = NodeData::new();
    p.bump(&mut node);

    if p.next().is_expr_first() {
        node.push_node(parse_atom(p));
    } else {
        node.push_error(ParseError::ExpectedExpr);
    }

    if p.next() == Token::RightParen {
        p.bump(&mut node);
    } else {
        node.push_error(ParseError::ExpectedRightParen);
    }

    node.set_node(Node::Group)
}

pub(crate) fn parse_atom(p: &mut ParseContext) -> NodeData {
    assert!(p.next().is_atom_expr_first());

    match p.next() {
        Token::Number => parse_number(p),
        Token::Ident => parse_name(p),
        Token::LeftParen => parse_group(p),
        _ => unreachable!(stringify!(is_atom_expr_first)),
    }
}

/// 添字付きなら true
fn parse_element_content(p: &mut ParseContext, node: &mut NodeData) -> bool {
    assert!(p.next().is_element_first());

    node.push_node(parse_atom(p));

    if p.next() == Token::LeftParen {
        parse_args(p, &mut node);

        if p.next() == Token::RightParen {
            p.bump(&mut node);
        } else {
            node.push_error(ParseError::ExpectedRightParen);
        }

        return true;
    }

    false
}

pub(crate) fn parse_element(p: &mut ParseContext) -> NodeData {
    let mut node = NodeData::new();
    parse_element_content(p, &mut node);
    node.set_node(Node::ElementExpr)
}

pub(crate) fn parse_element_or_command(p: &mut ParseContext, node: &mut NodeData) -> bool {
    parse_element_content(p, node) || !(p.at_eol() || p.next().is_expr_first())
}

pub(crate) fn parse_call(p: &mut ParseContext) -> NodeData {
    assert!(p.next().is_expr_first());

    match p.next() {
        Token::Number => parse_number(p),
        Token::Ident => parse_element(p),
        Token::LeftParen => parse_group(p),
        _ => {
            unreachable!(stringify!(is_expr_first));
        }
    }
}

pub(crate) fn parse_expr(p: &mut ParseContext) -> NodeData {
    parse_call(p)
}
