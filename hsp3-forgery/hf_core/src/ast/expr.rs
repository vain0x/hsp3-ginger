use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct AIntExpr {
    pub(crate) token: TokenData,
}

#[derive(Clone, Debug)]
pub(crate) struct AStrExpr {
    pub(crate) start_quote: TokenData,
    pub(crate) segments: Vec<TokenData>,
    pub(crate) end_quote_opt: Option<TokenData>,
}

#[derive(Clone, Debug)]
pub(crate) struct ANameExpr {
    pub(crate) token: TokenData,
}

#[derive(Clone, Debug)]
pub(crate) struct AGroupExpr {
    pub(crate) left_paren: TokenData,
    pub(crate) body_opt: Option<Box<AExpr>>,
    pub(crate) right_paren_opt: Option<TokenData>,
}

#[derive(Clone, Debug)]
pub(crate) struct ACallExpr {
    pub(crate) cal: ANameExpr,
    pub(crate) left_paren_opt: Option<TokenData>,
    pub(crate) args: Vec<AArg>,
    pub(crate) right_paren_opt: Option<TokenData>,
}

#[derive(Clone, Debug)]
pub(crate) enum AExpr {
    Int(AIntExpr),
    Str(AStrExpr),
    Name(ANameExpr),
    Group(AGroupExpr),
    Call(ACallExpr),
}

#[derive(Clone, Debug)]
pub(crate) struct AArg {
    pub(crate) expr_opt: Option<AExpr>,
    pub(crate) comma_opt: Option<TokenData>,
}
