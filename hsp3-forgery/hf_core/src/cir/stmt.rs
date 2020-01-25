use super::*;

#[derive(Debug)]
pub(crate) struct CFuncStmt {
    pub name: String,
    pub result_ty: CTy,
    pub body: Vec<CStmt>,
}

#[derive(Debug)]
pub(crate) enum CStmt {
    Return { arg_opt: Option<CExpr> },
    Func(CFuncStmt),
}
