use super::*;

impl Token {
    pub(crate) fn is_leading_trivia(self) -> bool {
        match self {
            Token::Eol | Token::Space | Token::Comment | Token::Other => true,
            _ => false,
        }
    }

    pub(crate) fn is_trailing_trivia(self) -> bool {
        match self {
            Token::Space
            | Token::Comment
            | Token::Other
            | Token::CharEnd
            | Token::StrEnd
            | Token::StrVerbatim
            | Token::StrEscape
            | Token::FloatPoint
            | Token::Fraction
            | Token::ExpChar
            | Token::ExpSign
            | Token::ExpDigit
            | Token::Binary
            | Token::Hex => true,
            _ => false,
        }
    }

    pub(crate) fn is_trivia(self) -> bool {
        self.is_leading_trivia() || self.is_trailing_trivia()
    }
}

#[derive(Clone)]
pub(crate) struct Trivia(TokenData);

impl From<Trivia> for TokenData {
    fn from(trivia: Trivia) -> TokenData {
        trivia.0
    }
}

impl From<TokenData> for Trivia {
    fn from(token: TokenData) -> Trivia {
        Trivia(token)
    }
}
