use super::*;

pub(crate) fn get_syntax_errors(syntax_root: &SyntaxRoot, d: &mut Diagnostics) {
    for element in syntax_root.node().descendant_elements() {
        match element {
            SyntaxElement::Token(token) => {
                if token.kind() == Token::Other {
                    d.push_error(Diagnostic::InvalidChars, token.location().range);
                }
            }
            SyntaxElement::Node(node) => match node.kind() {
                NodeKind::LabelLiteral => {
                    if !node.child_tokens().any(|token| {
                        token.kind() == Token::Ident || token.kind() == Token::IdentAtSign
                    }) {
                        d.push_error(Diagnostic::MissingLabelName, node.nontrivia_range());
                    }
                }
                NodeKind::GroupExpr => {
                    if let Some(left_paren) = node
                        .child_tokens()
                        .filter(|token| token.kind() == Token::LeftParen)
                        .next()
                    {
                        if node
                            .child_tokens()
                            .all(|token| token.kind() != Token::RightParen)
                        {
                            d.push_error(Diagnostic::UnclosedParen, left_paren.range());
                        }
                    }
                }
                NodeKind::CallExpr => {
                    if let Some(left_paren) = node
                        .child_tokens()
                        .filter(|token| token.kind() == Token::LeftParen)
                        .next()
                    {
                        if node
                            .child_tokens()
                            .all(|token| token.kind() != Token::RightParen)
                        {
                            d.push_error(Diagnostic::UnclosedParen, left_paren.range());
                        }
                    }
                }
                NodeKind::Param => {
                    if !node
                        .child_tokens()
                        .any(|token| token.kind() == Token::Ident)
                    {
                        d.push_error(Diagnostic::MissingParamType, node.nontrivia_range());
                    }
                }
                NodeKind::Other => {
                    d.push_error(Diagnostic::InvalidTokens, node.range());
                }
                _ => continue,
            },
        }
    }
}
