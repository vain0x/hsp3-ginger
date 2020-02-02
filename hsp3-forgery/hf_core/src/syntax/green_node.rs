use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeKind {
    LabelLiteral,
    StrLiteral,
    DoubleLiteral,
    IntLiteral,
    Ident,
    SystemVar,
    Param,
    Arg,
    CallExpr,
    BinaryExpr,
    AssignStmt,
    CommandStmt,
    LabelStmt,
    DeffuncPp,
    ModulePp,
    GlobalPp,
    UnknownPp,
    Root,
}

pub(crate) struct GreenNode {
    pub(crate) kind: NodeKind,
    pub(crate) children: Vec<GreenElement>,
}
