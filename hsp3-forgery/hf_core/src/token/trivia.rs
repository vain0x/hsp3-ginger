use super::*;

impl Token {
    pub(crate) fn is_leading_trivia(self) -> bool {
        self.is_trivia()
    }

    pub(crate) fn is_trailing_trivia(self) -> bool {
        self.is_trivia()
    }

    pub(crate) fn is_trivia(self) -> bool {
        self == Token::Space || self == Token::Comment || self == Token::Other
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
