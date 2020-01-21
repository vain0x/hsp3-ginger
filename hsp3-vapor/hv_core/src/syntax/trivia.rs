use super::*;

/// トリビア
///
/// ここでは構文的にあまり意味のない字句をトリビアを呼んでいる。
/// 空白やコメント、解釈できない文字など。
#[derive(Clone, Debug)]
pub(crate) struct Trivia(TokenData);

impl Trivia {
    pub(crate) fn as_token(&self) -> &TokenData {
        &self.0
    }

    pub(crate) fn into_token(self) -> TokenData {
        self.0
    }
}

impl From<TokenData> for Trivia {
    fn from(token: TokenData) -> Trivia {
        assert!(
            token.token().is_trivia(),
            "{:?} can't be a trivia",
            token.token()
        );

        Trivia(token)
    }
}

impl AsRef<TokenData> for Trivia {
    fn as_ref(&self) -> &TokenData {
        self.as_token()
    }
}

impl Token {
    pub(crate) fn is_leading_trivia(self) -> bool {
        self == Token::Eol || self.is_trailing_trivia()
    }

    pub(crate) fn is_trailing_trivia(self) -> bool {
        self == Token::Space || self == Token::Comment || self == Token::Other
    }

    pub(crate) fn is_trivia(self) -> bool {
        debug_assert!(!self.is_trailing_trivia() || self.is_leading_trivia());

        self.is_leading_trivia()
    }
}
