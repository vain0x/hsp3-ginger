use std::str::FromStr;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PPrivacy {
    Global,
    Local,
}

impl PPrivacy {
    fn parse(s: &str) -> Option<PPrivacy> {
        let it = match s {
            "global" => PPrivacy::Global,
            "local" => PPrivacy::Local,
            _ => return None,
        };
        Some(it)
    }
}

impl FromStr for PPrivacy {
    type Err = ();

    fn from_str(s: &str) -> Result<PPrivacy, ()> {
        PPrivacy::parse(s).ok_or(())
    }
}
