use super::*;

pub(crate) trait Ast: Sized {
    fn into_syntax(self) -> SyntaxNode;

    fn cast(syntax_node: SyntaxNode) -> Option<Self>;
}

pub(crate) struct ALabel(SyntaxNode);

impl Ast for ALabel {
    fn into_syntax(self) -> SyntaxNode {
        self.0
    }

    fn cast(syntax_node: SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::LabelLiteral {
            Some(ALabel(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AStr(SyntaxNode);

impl Ast for AStr {
    fn into_syntax(self) -> SyntaxNode {
        self.0
    }

    fn cast(syntax_node: SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::StrLiteral {
            Some(AStr(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AInt(SyntaxNode);

impl Ast for AInt {
    fn into_syntax(self) -> SyntaxNode {
        self.0
    }

    fn cast(syntax_node: SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::IntLiteral {
            Some(AInt(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AIdent(SyntaxNode);

impl Ast for AIdent {
    fn into_syntax(self) -> SyntaxNode {
        self.0
    }

    fn cast(syntax_node: SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Ident {
            Some(AIdent(syntax_node))
        } else {
            None
        }
    }
}
