use crate::token::TokenKind;

/// Binding power. 結合力 (演算子の優先順位付けに使うもの)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
pub(crate) struct Bp(i32);

impl Bp {
    // &, |, ^
    /// 論理演算
    pub(crate) const BOOL: Bp = Bp(1);
    // ==, !=, <, <=, >, >=
    /// 比較
    pub(crate) const COMPARE: Bp = Bp(2);
    // << >>
    /// シフト
    pub(crate) const SHIFT: Bp = Bp(3);
    // +, -
    /// 加減算
    pub(crate) const ADDSUB: Bp = Bp(4);
    // *, /, \\
    /// 乗除算・剰余
    pub(crate) const MULDIV: Bp = Bp(5);

    pub(crate) fn next(self) -> Bp {
        Bp(self.0 + 1)
    }

    pub(crate) fn from(token: TokenKind) -> Bp {
        assert!(token.is_infix_op());
        match token {
            TokenKind::And
            | TokenKind::AndAnd
            | TokenKind::Hat
            | TokenKind::PipePipe
            | TokenKind::Pipe => Bp::BOOL,
            TokenKind::Bang
            | TokenKind::BangEqual
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::LeftAngle
            | TokenKind::LeftEqual
            | TokenKind::RightAngle
            | TokenKind::RightEqual => Bp::COMPARE,
            TokenKind::LeftShift | TokenKind::RightShift => Bp::SHIFT,
            TokenKind::Minus | TokenKind::Plus => Bp::ADDSUB,
            TokenKind::Star | TokenKind::Backslash | TokenKind::Slash => Bp::MULDIV,
            _ => panic!("not an infix token"),
        }
    }
}
