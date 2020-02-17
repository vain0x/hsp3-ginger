use super::*;

pub(crate) struct AParam(SyntaxNode);

impl Ast for AParam {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::Param {
            Some(AParam(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct ADeffuncPp(SyntaxNode);

impl Ast for ADeffuncPp {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::DeffuncPp {
            Some(ADeffuncPp(syntax_node.clone()))
        } else {
            None
        }
    }
}

impl ADeffuncPp {
    pub(crate) fn is_local(&self) -> bool {
        self.syntax()
            .child_tokens()
            .any(|token| token.kind() == Token::Ident && token.text() == "local")
    }

    pub(crate) fn is_global(&self) -> bool {
        !self.is_local()
    }

    pub(crate) fn name(&self) -> Option<AIdent> {
        self.syntax()
            .child_nodes()
            .filter_map(|node| AIdent::cast(&node))
            .next()
    }

    pub(crate) fn params(&self) -> impl Iterator<Item = AParam> {
        self.syntax()
            .child_nodes()
            .filter_map(|node| AParam::cast(&node))
    }
}

pub(crate) enum AModuleName {
    Name(AIdent),
    Str(AStr),
}

impl AModuleName {
    pub(crate) fn to_string(&self) -> String {
        match self {
            AModuleName::Name(it) => it.to_string(),
            AModuleName::Str(it) => it.to_string(),
        }
    }
}

impl Ast for AModuleName {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            AModuleName::Name(it) => it.syntax(),
            AModuleName::Str(it) => it.syntax(),
        }
    }

    fn cast(node: &SyntaxNode) -> Option<Self> {
        if let Some(it) = AIdent::cast(node) {
            return Some(AModuleName::Name(it));
        }

        if let Some(it) = AStr::cast(node) {
            return Some(AModuleName::Str(it));
        }

        None
    }
}

pub(crate) struct AModulePp(SyntaxNode);

impl Ast for AModulePp {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::ModulePp {
            Some(AModulePp(syntax_node.clone()))
        } else {
            None
        }
    }
}

impl AModulePp {
    pub(crate) fn name(&self) -> Option<AModuleName> {
        self.syntax()
            .child_nodes()
            .filter_map(|node| AModuleName::cast(&node))
            .next()
    }
}

pub(crate) struct AGlobalPp(SyntaxNode);

impl Ast for AGlobalPp {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::GlobalPp {
            Some(AGlobalPp(syntax_node.clone()))
        } else {
            None
        }
    }
}
