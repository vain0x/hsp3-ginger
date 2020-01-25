use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct AIntExpr {
    pub token: TokenData,
}

#[derive(Clone, Debug)]
pub(crate) enum AExpr {
    Int(AIntExpr),
}
