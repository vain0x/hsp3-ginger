use super::*;
use std::rc::Rc;

pub(crate) trait Ast: Sized {
    fn into_syntax(self) -> Rc<SyntaxNode>;

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self>;
}

pub(crate) struct ALabel(Rc<SyntaxNode>);

impl Ast for ALabel {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::LabelLiteral {
            Some(ALabel(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AStr(Rc<SyntaxNode>);

impl Ast for AStr {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::StrLiteral {
            Some(AStr(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AInt(Rc<SyntaxNode>);

impl Ast for AInt {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::IntLiteral {
            Some(AInt(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct AIdent(Rc<SyntaxNode>);

impl Ast for AIdent {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Ident {
            Some(AIdent(syntax_node))
        } else {
            None
        }
    }
}
