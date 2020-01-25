use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct AReturnStmt {
    pub keyword: TokenData,
    pub result_opt: Option<AExpr>,
}

impl AReturnStmt {
    pub(crate) fn main_location(&self) -> SourceLocation {
        self.keyword.location
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ADeffuncStmt {
    pub hash: TokenData,
    pub keyword: TokenData,
    pub name_opt: Option<TokenData>,
}

impl ADeffuncStmt {
    pub(crate) fn name(&self) -> &str {
        self.name_opt
            .as_ref()
            .map(|token| token.text())
            .unwrap_or("_")
    }

    pub(crate) fn main_location(&self) -> SourceLocation {
        self.hash.location.unite(self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AModuleStmt {
    pub hash: TokenData,
    pub keyword: TokenData,
    pub name_opt: Option<TokenData>,
}

impl AModuleStmt {
    pub(crate) fn main_location(&self) -> SourceLocation {
        self.hash.location.unite(self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AGlobalStmt {
    pub hash: TokenData,
    pub keyword: TokenData,
}

impl AGlobalStmt {
    pub(crate) fn main_location(&self) -> SourceLocation {
        self.hash.location.unite(self.keyword.location)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AStmt {
    Return(AReturnStmt),
    Module(AModuleStmt),
    Global(AGlobalStmt),
    Deffunc(ADeffuncStmt),
    UnknownPreprocessor { hash: TokenData },
}

impl AStmt {
    pub(crate) fn main_location(&self) -> SourceLocation {
        match self {
            AStmt::Return(stmt) => stmt.main_location(),
            AStmt::Module(stmt) => stmt.main_location(),
            AStmt::Global(stmt) => stmt.main_location(),
            AStmt::Deffunc(stmt) => stmt.main_location(),
            AStmt::UnknownPreprocessor { hash, .. } => hash.location,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ARoot {
    pub children: Vec<AStmt>,
    pub errors: Vec<SyntaxError>,
}
