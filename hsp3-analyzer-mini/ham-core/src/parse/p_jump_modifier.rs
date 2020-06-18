use std::str::FromStr;

/// oncmd や button の直後の goto/gosub
#[derive(Clone, Copy)]
pub(crate) enum PJumpModifier {
    Goto,
    Gosub,
}

impl PJumpModifier {
    fn parse(s: &str) -> Option<PJumpModifier> {
        let it = match s {
            "goto" => PJumpModifier::Goto,
            "gosub" => PJumpModifier::Gosub,
            _ => return None,
        };
        Some(it)
    }
}

impl FromStr for PJumpModifier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        PJumpModifier::parse(s).ok_or(())
    }
}
