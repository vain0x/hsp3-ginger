#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ParamTy {
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

impl ParamTy {
    /// 実引数を受け取るか？
    pub(crate) fn takes_arg(self) -> bool {
        match self {
            ParamTy::Local | ParamTy::Nullptr => false,
            _ => true,
        }
    }
}

impl std::str::FromStr for ParamTy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PARAM_TY_TABLE
            .iter()
            .filter_map(|&(param_ty, word)| if word == s { Some(param_ty) } else { None })
            .next()
            .ok_or(())
    }
}

pub(crate) static PARAM_TY_TABLE: &[(ParamTy, &str)] = &[
    (ParamTy::Label, "label"),
    (ParamTy::Str, "str"),
    (ParamTy::Double, "double"),
    (ParamTy::Int, "int"),
    (ParamTy::Var, "var"),
    (ParamTy::Modvar, "modvar"),
    (ParamTy::Array, "array"),
    (ParamTy::Local, "local"),
    (ParamTy::Sptr, "sptr"),
    (ParamTy::Wptr, "wptr"),
    (ParamTy::Nullptr, "nullptr"),
];
