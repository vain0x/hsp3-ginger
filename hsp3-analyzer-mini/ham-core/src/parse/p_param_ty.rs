use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PParamTy {
    Label,
    Str,
    Double,
    Int,
    Var,
    Array,
    Modvar,
    Local,
    Nullptr,
    WStr,
    Float,
    SPtr,
    WPtr,
    Comobj,
    Bmscr,
    PRefstr,
    PExinfo,
}

impl PParamTy {
    fn parse(s: &str) -> Option<PParamTy> {
        let param_ty = match s {
            "label" => PParamTy::Label,
            "str" => PParamTy::Str,
            "double" => PParamTy::Double,
            "int" => PParamTy::Int,
            "var" => PParamTy::Var,
            "array" => PParamTy::Array,
            "modvar" => PParamTy::Modvar,
            "local" => PParamTy::Local,
            "nullptr" => PParamTy::Nullptr,
            "wstr" => PParamTy::WStr,
            "float" => PParamTy::Float,
            "sptr" => PParamTy::SPtr,
            "wptr" => PParamTy::WPtr,
            "comobj" => PParamTy::Comobj,
            "bmscr" => PParamTy::Bmscr,
            "prefstr" => PParamTy::PRefstr,
            "pexinfo" => PParamTy::PExinfo,
            _ => return None,
        };
        Some(param_ty)
    }
}

impl FromStr for PParamTy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        PParamTy::parse(s).ok_or(())
    }
}
