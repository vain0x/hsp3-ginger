use super::*;

pub(crate) struct AGroupExpr(SyntaxNode);

impl Ast for AGroupExpr {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::GroupExpr {
            Some(AGroupExpr(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct ACallExpr(SyntaxNode);

impl Ast for ACallExpr {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::CallExpr {
            Some(ACallExpr(syntax_node.clone()))
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
    Group(AGroupExpr),
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
    fn syntax(&self) -> &SyntaxNode {
        match self {
            AExpr::Label(a) => a.syntax(),
            AExpr::Str(a) => a.syntax(),
            AExpr::Int(a) => a.syntax(),
            AExpr::Ident(a) => a.syntax(),
            AExpr::Group(a) => a.syntax(),
            AExpr::Call(a) => a.syntax(),
        }
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if let Some(a) = ALabel::cast(syntax_node) {
            return Some(AExpr::Label(a));
        }

        if let Some(a) = AStr::cast(syntax_node) {
            return Some(AExpr::Str(a));
        }

        if let Some(a) = AInt::cast(syntax_node) {
            return Some(AExpr::Int(a));
        }

        if let Some(a) = AIdent::cast(syntax_node) {
            return Some(AExpr::Ident(a));
        }

        if let Some(a) = AGroupExpr::cast(syntax_node) {
            return Some(AExpr::Group(a));
        }

        if let Some(a) = ACallExpr::cast(syntax_node) {
            return Some(AExpr::Call(a));
        }

        None
    }
}

pub(crate) struct AArg(SyntaxNode);

impl Ast for AArg {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Arg {
            Some(AArg(syntax_node.clone()))
        } else {
            None
        }
    }
}
