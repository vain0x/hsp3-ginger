use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct ALabelStmt {
    pub(crate) label: ALabel,
    pub(crate) sep_opt: Option<TokenData>,
}

impl ALabelStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.label.location()
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AAssignStmt {
    pub(crate) left: TokenData,
    pub(crate) equal: TokenData,
    pub(crate) right_opt: Option<AExpr>,
    pub(crate) sep_opt: Option<TokenData>,
}

impl AAssignStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.equal.location.clone()
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ACommandStmt {
    pub(crate) command: TokenData,
    pub(crate) args: Vec<AArg>,
    pub(crate) sep_opt: Option<TokenData>,
}

impl ACommandStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.command.location.clone()
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AReturnStmt {
    pub(crate) keyword: TokenData,
    pub(crate) result_opt: Option<AExpr>,
    pub(crate) sep_opt: Option<TokenData>,
}

impl AReturnStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.keyword.location.clone()
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ADeffuncStmt {
    pub(crate) hash: TokenData,
    pub(crate) keyword: TokenData,
    pub(crate) name_opt: Option<TokenData>,
    pub(crate) sep_opt: Option<TokenData>,
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

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AModuleStmt {
    pub(crate) hash: TokenData,
    pub(crate) keyword: TokenData,
    pub(crate) name_opt: Option<TokenData>,
    pub(crate) sep_opt: Option<TokenData>,
}

impl AModuleStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone().unite(&self.keyword.location)
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AGlobalStmt {
    pub(crate) hash: TokenData,
    pub(crate) keyword: TokenData,
    pub(crate) sep_opt: Option<TokenData>,
}

impl AGlobalStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone().unite(&self.keyword.location)
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AUnknownPpStmt {
    pub(crate) hash: TokenData,
    pub(crate) sep_opt: Option<TokenData>,
}

impl AUnknownPpStmt {
    pub(crate) fn main_location(&self) -> Location {
        self.hash.location.clone()
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        self.sep_opt.as_ref()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AStmt {
    Label(ALabelStmt),
    Assign(AAssignStmt),
    Command(ACommandStmt),
    Return(AReturnStmt),
    Module(AModuleStmt),
    Global(AGlobalStmt),
    Deffunc(ADeffuncStmt),
    UnknownPp(AUnknownPpStmt),
}

impl AStmt {
    pub(crate) fn main_location(&self) -> Location {
        match self {
            AStmt::Label(stmt) => stmt.main_location(),
            AStmt::Assign(stmt) => stmt.main_location(),
            AStmt::Command(stmt) => stmt.main_location(),
            AStmt::Return(stmt) => stmt.main_location(),
            AStmt::Module(stmt) => stmt.main_location(),
            AStmt::Global(stmt) => stmt.main_location(),
            AStmt::Deffunc(stmt) => stmt.main_location(),
            AStmt::UnknownPp(stmt) => stmt.main_location(),
        }
    }

    pub(crate) fn sep_opt(&self) -> Option<&TokenData> {
        match self {
            AStmt::Label(stmt) => stmt.sep_opt(),
            AStmt::Assign(stmt) => stmt.sep_opt(),
            AStmt::Command(stmt) => stmt.sep_opt(),
            AStmt::Return(stmt) => stmt.sep_opt(),
            AStmt::Module(stmt) => stmt.sep_opt(),
            AStmt::Global(stmt) => stmt.sep_opt(),
            AStmt::Deffunc(stmt) => stmt.sep_opt(),
            AStmt::UnknownPp(stmt) => stmt.sep_opt(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ARoot {
    pub(crate) children: Vec<AStmt>,
    pub(crate) errors: Vec<SyntaxError>,
}
