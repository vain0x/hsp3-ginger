use super::*;

#[derive(Clone, Debug)]
pub(crate) enum AStmtNode {
    Label(ALabelStmt),
    Assign(AAssignStmt),
    Command(ACommandStmt),
}

#[derive(Clone, Debug)]
pub(crate) struct AFnNode {
    pub(crate) deffunc_stmt: ADeffuncStmt,
}

#[derive(Clone, Debug)]
pub(crate) struct AModuleNode {
    pub(crate) module_stmt: AModuleStmt,
    pub(crate) global_stmt_opt: Option<AGlobalStmt>,
}

#[derive(Clone, Debug)]
pub(crate) struct ARootNode {
    pub(crate) errors: Vec<SyntaxError>,
}

#[derive(Clone, Debug)]
pub(crate) enum ANode {
    Stmt(AStmtNode),
    Fn(AFnNode),
    Module(AModuleNode),
    Root(ARootNode),
}

impl ANode {
    pub(crate) fn is_root(&self) -> bool {
        match self {
            ANode::Root(..) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ANodeData {
    pub(crate) node: ANode,
    pub(crate) children: Vec<ANodeData>,
}

impl From<ALabelStmt> for ANodeData {
    fn from(stmt: ALabelStmt) -> ANodeData {
        ANodeData {
            node: ANode::Stmt(AStmtNode::Label(stmt)),
            children: vec![],
        }
    }
}

impl From<AAssignStmt> for ANodeData {
    fn from(stmt: AAssignStmt) -> ANodeData {
        ANodeData {
            node: ANode::Stmt(AStmtNode::Assign(stmt)),
            children: vec![],
        }
    }
}

impl From<ACommandStmt> for ANodeData {
    fn from(stmt: ACommandStmt) -> ANodeData {
        ANodeData {
            node: ANode::Stmt(AStmtNode::Command(stmt)),
            children: vec![],
        }
    }
}
