use super::*;

pub(crate) struct AAssignStmt(SyntaxNode);

impl Ast for AAssignStmt {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::AssignStmt {
            Some(AAssignStmt(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct ACommandStmt(SyntaxNode);

impl Ast for ACommandStmt {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::CommandStmt {
            Some(ACommandStmt(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) struct ALabelStmt(SyntaxNode);

impl Ast for ALabelStmt {
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if syntax_node.kind() == NodeKind::LabelStmt {
            Some(ALabelStmt(syntax_node.clone()))
        } else {
            None
        }
    }
}

pub(crate) enum AStmt {
    Assign(AAssignStmt),
    Command(ACommandStmt),
    Label(ALabelStmt),
}

impl Ast for AStmt {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            AStmt::Assign(a) => a.syntax(),
            AStmt::Command(a) => a.syntax(),
            AStmt::Label(a) => a.syntax(),
        }
    }

    fn cast(syntax_node: &SyntaxNode) -> Option<Self> {
        if let Some(a) = AAssignStmt::cast(syntax_node) {
            return Some(AStmt::Assign(a));
        }

        if let Some(a) = ACommandStmt::cast(syntax_node) {
            return Some(AStmt::Command(a));
        }

        if let Some(a) = ALabelStmt::cast(syntax_node) {
            return Some(AStmt::Label(a));
        }

        None
    }
}
