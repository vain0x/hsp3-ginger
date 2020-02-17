use super::*;

pub(crate) trait Ast: Sized {
    fn syntax(&self) -> &SyntaxNode;

    fn cast(syntax_node: &SyntaxNode) -> Option<Self>;
}

pub(crate) struct ALabel(SyntaxNode);

impl Ast for ALabel {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::LabelLiteral {
            Some(ALabel(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct AStr(SyntaxNode);

impl AStr {
    pub(crate) fn to_string(&self) -> String {
        let mut s = String::new();

        for token in self.syntax().child_tokens() {
            match token.kind() {
                Token::StrVerbatim => {
                    s += token.text();
                }
                Token::StrEscape => {
                    // FIXME: エスケープのデコードを実装
                    s += token.text();
                }
                _ => {}
            }
        }

        s
    }
}

impl Ast for AStr {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::StrLiteral {
            Some(AStr(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct AInt(SyntaxNode);

impl Ast for AInt {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::IntLiteral {
            Some(AInt(syntax_node.clone()))
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct AIdent(SyntaxNode);

impl Ast for AIdent {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Ident {
            Some(AIdent(syntax_node.clone()))
        } else {
            None
        }
    }
}

impl AIdent {
    pub(crate) fn is_qualified(&self) -> bool {
        self.syntax()
            .child_tokens()
            .any(|token| token.kind() == Token::IdentAtSign)
    }

    pub(crate) fn unqualified_name(&self) -> String {
        self.syntax()
            .child_tokens()
            .filter_map(|token| {
                if token.kind() == Token::Ident {
                    Some(token.text().to_string())
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(String::new())
    }

    pub(crate) fn scope_name(&self) -> Option<String> {
        self.syntax()
            .child_tokens()
            .filter_map(|token| {
                if token.kind() == Token::IdentScope {
                    Some(token.text().to_string())
                } else {
                    None
                }
            })
            .next()
    }

    pub(crate) fn to_string(&self) -> String {
        let mut out = String::new();
        for token in self.0.child_tokens() {
            match token.kind() {
                Token::Ident | Token::IdentAtSign | Token::IdentScope => out += token.text(),
                _ => {}
            }
        }
        out
    }
}
