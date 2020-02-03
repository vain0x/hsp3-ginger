use super::*;
use std::rc::Rc;

pub(crate) struct AAssignStmt(Rc<SyntaxNode>);

impl Ast for AAssignStmt {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::AssignStmt {
            Some(AAssignStmt(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct ACommandStmt(Rc<SyntaxNode>);

impl Ast for ACommandStmt {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::CommandStmt {
            Some(ACommandStmt(syntax_node))
        } else {
            None
        }
    }
}

pub(crate) struct ALabelStmt(Rc<SyntaxNode>);

impl Ast for ALabelStmt {
    fn into_syntax(self) -> Rc<SyntaxNode> {
        self.0
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if syntax_node.kind() == NodeKind::LabelStmt {
            Some(ALabelStmt(syntax_node))
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
    fn into_syntax(self) -> Rc<SyntaxNode> {
        match self {
            AStmt::Assign(a) => a.into_syntax(),
            AStmt::Command(a) => a.into_syntax(),
            AStmt::Label(a) => a.into_syntax(),
        }
    }

    fn cast(syntax_node: Rc<SyntaxNode>) -> Option<Self> {
        if let Some(a) = AAssignStmt::cast(syntax_node.clone()) {
            return Some(AStmt::Assign(a));
        }

        if let Some(a) = ACommandStmt::cast(syntax_node.clone()) {
            return Some(AStmt::Command(a));
        }

        if let Some(a) = ALabelStmt::cast(syntax_node.clone()) {
            return Some(AStmt::Label(a));
        }

        None
    }
}
