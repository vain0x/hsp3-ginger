#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub(crate) fn parse(s: &str) -> Option<PParamTy> {
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

    pub(crate) fn to_str(self) -> &'static str {
        match self {
            PParamTy::Label => "label",
            PParamTy::Str => "str",
            PParamTy::Double => "double",
            PParamTy::Int => "int",
            PParamTy::Var => "var",
            PParamTy::Array => "array",
            PParamTy::Modvar => "modvar",
            PParamTy::Local => "local",
            PParamTy::Nullptr => "nullptr",
            PParamTy::WStr => "wstr",
            PParamTy::Float => "float",
            PParamTy::SPtr => "sptr",
            PParamTy::WPtr => "wptr",
            PParamTy::Comobj => "comobj",
            PParamTy::Bmscr => "bmscr",
            PParamTy::PRefstr => "prefstr",
            PParamTy::PExinfo => "pexinfo",
            PParamTy::Hwnd => "hwnd",
            PParamTy::Hdc => "hdc",
            PParamTy::HInst => "hinst",
        }
    }

    pub(crate) fn category(self) -> PParamCategory {
        match self {
            PParamTy::Str
            | PParamTy::Double
            | PParamTy::Int
            | PParamTy::Label
            | PParamTy::WStr
            | PParamTy::Float
            | PParamTy::SPtr
            | PParamTy::WPtr => PParamCategory::ByValue,

            PParamTy::Var | PParamTy::Array | PParamTy::Modvar | PParamTy::Comobj => {
                PParamCategory::ByRef
            }

            PParamTy::Local => PParamCategory::Local,

            PParamTy::Nullptr
            | PParamTy::Bmscr
            | PParamTy::PRefstr
            | PParamTy::PExinfo
            | PParamTy::Hwnd
            | PParamTy::Hdc
            | PParamTy::HInst => PParamCategory::Auto,
        }
    }

    pub(crate) fn take_arg(self) -> bool {
        match self.category() {
            PParamCategory::ByValue | PParamCategory::ByRef => true,
            PParamCategory::Local | PParamCategory::Auto => false,
        }
    }

    pub(crate) fn is_by_ref(self) -> bool {
        self.category() == PParamCategory::ByRef
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PParamCategory {
    /// 値渡し。エイリアスは書き換え不可。
    ByValue,

    /// 参照渡し。エイリアスへの変更は引数に影響する。
    ByRef,

    /// ローカル変数。(参照渡しと違って、エイリアスへの変更は引数に影響しない。)
    Local,

    /// システム変数の値が自動で渡される。
    Auto,
}
