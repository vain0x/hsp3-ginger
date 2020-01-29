use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct AIntExpr {
    pub(crate) token: TokenData,
}

#[derive(Clone, Debug)]
pub(crate) enum AExpr {
    Int(AIntExpr),
}

#[derive(Clone, Debug)]
pub(crate) struct AArg {
    pub(crate) expr_opt: Option<AExpr>,
    pub(crate) comma_opt: Option<TokenData>,
}
