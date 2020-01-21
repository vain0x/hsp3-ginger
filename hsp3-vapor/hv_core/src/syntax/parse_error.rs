#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ParseError {
    ExpectedCommaOrEol,
    ExpectedExpr,
    ExpectedFatArrow,
    ExpectedIdent,
    ExpectedLeftBrace,
    ExpectedRightBrace,
    ExpectedRightParen,
    UnexpectedChars,
}
