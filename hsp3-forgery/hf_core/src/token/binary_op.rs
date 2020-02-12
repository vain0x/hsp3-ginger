use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOpLevel {
    /// `|`, `||`, `^`
    Or,
    /// `&`, `&&`
    And,
    /// `<<`, `>>`
    Shift,
    /// Ordering. `==`, `<`, etc.
    Ord,
    /// Additive. `+`, `-`
    Add,
    /// Multitive. `*`, `/`, `\`
    Mul,
}

impl BinaryOpLevel {
    pub(crate) const LOWEST: BinaryOpLevel = BinaryOpLevel::Or;

    pub(crate) fn next(self) -> Option<BinaryOpLevel> {
        match self {
            BinaryOpLevel::Or => Some(BinaryOpLevel::And),
            BinaryOpLevel::And => Some(BinaryOpLevel::Shift),
            BinaryOpLevel::Shift => Some(BinaryOpLevel::Ord),
            BinaryOpLevel::Ord => Some(BinaryOpLevel::Add),
            BinaryOpLevel::Add => Some(BinaryOpLevel::Mul),
            BinaryOpLevel::Mul => None,
        }
    }
}

pub(crate) static BINARY_OP_TABLE: &[(BinaryOpLevel, Token)] = &[
    (BinaryOpLevel::Or, Token::PipePipe),
    (BinaryOpLevel::Or, Token::Pipe),
    (BinaryOpLevel::Or, Token::Hat),
    (BinaryOpLevel::And, Token::AndAnd),
    (BinaryOpLevel::And, Token::And),
    (BinaryOpLevel::Shift, Token::LeftShift),
    (BinaryOpLevel::Shift, Token::RightShift),
    (BinaryOpLevel::Ord, Token::EqualEqual),
    (BinaryOpLevel::Ord, Token::Equal),
    (BinaryOpLevel::Ord, Token::BangEqual),
    (BinaryOpLevel::Ord, Token::Bang),
    (BinaryOpLevel::Ord, Token::LeftAngle),
    (BinaryOpLevel::Ord, Token::LeftEqual),
    (BinaryOpLevel::Ord, Token::RightAngle),
    (BinaryOpLevel::Ord, Token::RightEqual),
    (BinaryOpLevel::Add, Token::Plus),
    (BinaryOpLevel::Add, Token::Minus),
    (BinaryOpLevel::Mul, Token::Star),
    (BinaryOpLevel::Mul, Token::Slash),
    (BinaryOpLevel::Mul, Token::Backslash),
];
