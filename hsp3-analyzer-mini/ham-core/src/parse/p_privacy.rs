/// プライバシー (`#define` などにつける global/local キーワード)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PPrivacy {
    Global,
    Local,
}

impl PPrivacy {
    pub(crate) fn parse(s: &str) -> Option<PPrivacy> {
        let it = match s {
            "global" => PPrivacy::Global,
            "local" => PPrivacy::Local,
            _ => return None,
        };
        Some(it)
    }
}
