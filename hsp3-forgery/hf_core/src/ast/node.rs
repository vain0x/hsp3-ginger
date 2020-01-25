use super::*;

#[derive(Clone, Debug)]
pub(crate) enum AStmtNode {
    Assign(AAssignStmt),
    Return(AReturnStmt),
}

#[derive(Clone, Debug)]
pub(crate) struct AFnNode {
    pub deffunc_stmt: ADeffuncStmt,
}

#[derive(Clone, Debug)]
pub(crate) struct AModuleNode {
    pub module_stmt: AModuleStmt,
    pub global_stmt_opt: Option<AGlobalStmt>,
}

#[derive(Clone, Debug)]
pub(crate) struct ARootNode {
    pub errors: Vec<SyntaxError>,
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
    pub node: ANode,
    pub children: Vec<ANodeData>,
}

impl From<AAssignStmt> for ANodeData {
    fn from(stmt: AAssignStmt) -> ANodeData {
        ANodeData {
            node: ANode::Stmt(AStmtNode::Assign(stmt)),
            children: vec![],
        }
    }
}
impl From<AReturnStmt> for ANodeData {
    fn from(return_stmt: AReturnStmt) -> ANodeData {
        ANodeData {
            node: ANode::Stmt(AStmtNode::Return(return_stmt)),
            children: vec![],
        }
    }
}
