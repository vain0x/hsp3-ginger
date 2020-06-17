use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AParamTy {
    Label,
    Str,
    Double,
    Int,
    Var,
    Modvar,
    Array,
    Local,
    Sptr,
    Wptr,
    Nullptr,
}

impl AParamTy {
    /// 実引数を受け取るか？

    #[allow(unused)]
    pub(crate) fn takes_arg(self) -> bool {
        match self {
            AParamTy::Local | AParamTy::Nullptr => false,
            _ => true,
        }
    }
}

impl FromStr for AParamTy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        PARAM_TY_TABLE
            .iter()
            .find_map(|&(param_ty, word)| if word == s { Some(param_ty) } else { None })
            .ok_or(())
    }
}

static PARAM_TY_TABLE: &[(AParamTy, &str)] = &[
    (AParamTy::Label, "label"),
    (AParamTy::Str, "str"),
    (AParamTy::Double, "double"),
    (AParamTy::Int, "int"),
    (AParamTy::Var, "var"),
    (AParamTy::Modvar, "modvar"),
    (AParamTy::Array, "array"),
    (AParamTy::Local, "local"),
    (AParamTy::Sptr, "sptr"),
    (AParamTy::Wptr, "wptr"),
    (AParamTy::Nullptr, "nullptr"),
];
