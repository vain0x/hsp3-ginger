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
