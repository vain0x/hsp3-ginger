use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct AAssignStmt {
    pub(crate) left: TokenData,
    pub(crate) equal: TokenData,
    pub(crate) right_opt: Option<AExpr>,
}

impl AAssignStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.equal.location.clone()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ACommandStmt {
    pub(crate) command: TokenData,
    pub(crate) args: Vec<AArg>,
}

impl ACommandStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.command.location.clone()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AReturnStmt {
    pub(crate) keyword: TokenData,
    pub(crate) result_opt: Option<AExpr>,
}

impl AReturnStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.keyword.location.clone()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ADeffuncStmt {
    pub(crate) hash: TokenData,
    pub(crate) keyword: TokenData,
    pub(crate) name_opt: Option<TokenData>,
}

impl ADeffuncStmt {
    pub(crate) fn name(&self) -> &str {
        self.name_opt
            .as_ref()
            .map(|token| token.text())
            .unwrap_or("_")
    }

    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone().unite(&self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AModuleStmt {
    pub(crate) hash: TokenData,
    pub(crate) keyword: TokenData,
    pub(crate) name_opt: Option<TokenData>,
}

impl AModuleStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone().unite(&self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AGlobalStmt {
    pub hash: TokenData,
    pub keyword: TokenData,
}

impl AGlobalStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone().unite(&self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AStmt {
    Label(ALabel),
    Assign(AAssignStmt),
    Command(ACommandStmt),
    Return(AReturnStmt),
    Module(AModuleStmt),
    Global(AGlobalStmt),
    Deffunc(ADeffuncStmt),
    UnknownPreprocessor { hash: TokenData },
}

impl AStmt {
    pub(crate) fn main_location(&self) -> Location {
        match self {
            AStmt::Label(label) => label.location(),
            AStmt::Assign(stmt) => stmt.main_location(),
            AStmt::Command(stmt) => stmt.main_location(),
            AStmt::Return(stmt) => stmt.main_location(),
            AStmt::Module(stmt) => stmt.main_location(),
            AStmt::Global(stmt) => stmt.main_location(),
            AStmt::Deffunc(stmt) => stmt.main_location(),
            AStmt::UnknownPreprocessor { hash, .. } => hash.location.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ARoot {
    pub children: Vec<AStmt>,
    pub errors: Vec<SyntaxError>,
}
