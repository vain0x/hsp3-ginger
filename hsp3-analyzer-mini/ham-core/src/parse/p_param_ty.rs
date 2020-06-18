use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PParamTy {
    Str,
    Double,
    Int,
    Var,

    // `#deffunc` など
    Label,
    Array,
    Modvar,
    Local,

    // `#func` など
    Nullptr,
    WStr,
    Float,
    SPtr,
    WPtr,
    Comobj,
    Bmscr,
    PRefstr,
    PExinfo,

    // `#comfunc`
    Hwnd,
    Hdc,
    HInst,
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
            "hwnd" => PParamTy::Hwnd,
            "hdc" => PParamTy::Hdc,
            "hinst" => PParamTy::HInst,
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
