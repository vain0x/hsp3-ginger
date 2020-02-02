use super::*;

#[derive(Clone, Debug)]
pub(crate) enum ALabel {
    /// `*foo`, etc.
    Name { star: TokenData, ident: TokenData },
    /// `*@f`, etc.
    Anonymous {
        star: TokenData,
        at_sign: TokenData,
        ident_opt: Option<TokenData>,
    },
    /// ラベルしか出現しない文脈で `*` が見えたらとりあえずラベルとして扱う。
    /// その後ろに名前も `@` も出てこないケースがこれ。
    StarOnly { star: TokenData },
}

impl ALabel {
    pub(crate) fn location(&self) -> Location {
        match self {
            ALabel::Name { star, ident } => star.location.clone().unite(&ident.location),
            ALabel::Anonymous {
                star,
                ident_opt: Some(ident),
                ..
            } => star.location.clone().unite(&ident.location),
            ALabel::Anonymous { star, at_sign, .. } => {
                star.location.clone().unite(&at_sign.location)
            }
            ALabel::StarOnly { star } => star.location.clone(),
        }
    }

    pub(crate) fn star(&self) -> &TokenData {
        match self {
            ALabel::Name { star, .. }
            | ALabel::Anonymous { star, .. }
            | ALabel::StarOnly { star, .. } => star,
        }
    }
}

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
    Label(ALabel),
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
