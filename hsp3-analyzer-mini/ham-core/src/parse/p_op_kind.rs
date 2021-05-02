use crate::token::TokenKind;

/// トークンを演算子として解釈するとき、どのような構文にパースされるか？
#[derive(PartialEq, Eq)]
pub(crate) enum POpKind {
    /// 中置のみ
    Infix,
    /// 代入のみ
    Assign,
    /// 中置または代入
    InfixOrAssign,
    /// 前置、中置、代入
    PrefixOrInfixOrAssign,
}

impl TokenKind {
    pub(crate) fn to_op_kind(self) -> Option<POpKind> {
        let it = match self {
            TokenKind::Minus | TokenKind::Star => POpKind::PrefixOrInfixOrAssign,
            TokenKind::LeftAngle
            | TokenKind::RightAngle
            | TokenKind::AndAnd
            | TokenKind::BangEqual
            | TokenKind::EqualEqual
            | TokenKind::LeftEqual
            | TokenKind::RightEqual
            | TokenKind::PipePipe => POpKind::Infix,
            TokenKind::And
            | TokenKind::Backslash
            | TokenKind::Bang
            | TokenKind::Equal
            | TokenKind::Hat
            | TokenKind::LeftShift
            | TokenKind::Pipe
            | TokenKind::Plus
            | TokenKind::RightShift
            | TokenKind::Slash => POpKind::InfixOrAssign,
            TokenKind::AndEqual
            | TokenKind::BackslashEqual
            | TokenKind::HatEqual
            | TokenKind::MinusEqual
            | TokenKind::MinusMinus
            | TokenKind::PipeEqual
            | TokenKind::PlusEqual
            | TokenKind::PlusPlus
            | TokenKind::SlashEqual
            | TokenKind::StarEqual => POpKind::Assign,
            _ => return None,
        };
        Some(it)
    }

    pub(crate) fn is_infix_op(self) -> bool {
        match self.to_op_kind() {
            None | Some(POpKind::Assign) => false,
            Some(POpKind::Infix)
            | Some(POpKind::InfixOrAssign)
            | Some(POpKind::PrefixOrInfixOrAssign) => true,
        }
    }

    pub(crate) fn is_assign_op(self) -> bool {
        match self.to_op_kind() {
            None | Some(POpKind::Infix) => false,
            Some(POpKind::Assign)
            | Some(POpKind::InfixOrAssign)
            | Some(POpKind::PrefixOrInfixOrAssign) => true,
        }
    }
}
