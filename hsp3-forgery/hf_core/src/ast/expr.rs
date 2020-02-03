use super::*;
use std::rc::Rc;

pub(crate) struct ACallExpr(Rc<SyntaxNode>);

impl Ast for ACallExpr {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::CallExpr {
            Some(ACallExpr(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) enum AExpr {
    Label(ALabel),
    Str(AStr),
    Int(AInt),
    Ident(AIdent),
    Call(ACallExpr),
}

impl From<ALabel> for AExpr {
    fn from(it: ALabel) -> Self {
        AExpr::Label(it)
    }
}

impl From<AStr> for AExpr {
    fn from(it: AStr) -> Self {
        AExpr::Str(it)
    }
}

impl From<AInt> for AExpr {
    fn from(it: AInt) -> Self {
        AExpr::Int(it)
    }
}

impl From<AIdent> for AExpr {
    fn from(it: AIdent) -> Self {
        AExpr::Ident(it)
    }
}

impl From<ACallExpr> for AExpr {
    fn from(it: ACallExpr) -> Self {
        AExpr::Call(it)
    }
}

impl Ast for AExpr {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        match self {
            AExpr::Label(a) => a.into_syntax(),
            AExpr::Str(a) => a.into_syntax(),
            AExpr::Int(a) => a.into_syntax(),
            AExpr::Ident(a) => a.into_syntax(),
            AExpr::Call(a) => a.into_syntax(),
        }
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if let Some(a) = ALabel::cast(syntax_node.clone()) {
            return Some(AExpr::Label(a));
        }

        if let Some(a) = AStr::cast(syntax_node.clone()) {
            return Some(AExpr::Str(a));
        }

        if let Some(a) = AInt::cast(syntax_node.clone()) {
            return Some(AExpr::Int(a));
        }

        if let Some(a) = AIdent::cast(syntax_node.clone()) {
            return Some(AExpr::Ident(a));
        }

        if let Some(a) = ACallExpr::cast(syntax_node.clone()) {
            return Some(AExpr::Call(a));
        }

        None
    }
}

pub(crate) struct AArg(Rc<SyntaxNode>);

impl Ast for AArg {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Arg {
            Some(AArg(syntax_node))
        } else {
            None
        }
    }
}
