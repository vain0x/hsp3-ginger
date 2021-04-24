#[derive(Copy, Clone, Debug)]
pub(crate) enum PConstTy {
    Double,
    Int,
}

impl PConstTy {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        let it = match s {
            "double" => PConstTy::Double,
            "int" => PConstTy::Int,
            _ => return None,
        };
        Some(it)
    }
}
