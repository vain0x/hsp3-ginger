/// oncmd や button の直後の goto/gosub
#[derive(Clone, Copy)]
pub(crate) enum PJumpModifier {
    Goto,
    Gosub,
}

impl PJumpModifier {
    pub(crate) fn parse(s: &str) -> Option<PJumpModifier> {
        let it = match s {
            "goto" => PJumpModifier::Goto,
            "gosub" => PJumpModifier::Gosub,
            _ => return None,
        };
        Some(it)
    }
}
