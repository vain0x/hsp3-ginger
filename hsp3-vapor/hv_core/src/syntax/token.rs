use super::*;

/// 字句の種類
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Token {
    Eof,
    Eol,
    Space,
    Comment,
    Number,
    Ident,
    Other,

    // キーワード
    If,
    Else,
    Repeat,
    Loop,
    Break,
    Continue,
    Return,

    // 約物
    /// (
    LeftParen,
    /// )
    RightParen,
    /// <
    LeftAngle,
    /// >
    RightAngle,
    /// {
    LeftBrace,
    /// }
    RightBrace,
    /// :
    Colon,
    /// ,
    Comma,
    /// .
    Dot,
    /// =
    Equal,
    /// -
    Minus,
}

/// 字句のデータ
#[derive(Clone, Debug)]
pub(crate) struct TokenData {
    token: Token,
    text: String,
}

impl TokenData {
    pub(crate) fn new(token: Token, text: String) -> Self {
        TokenData { token, text }
    }

    pub(crate) fn token(&self) -> Token {
        self.token
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

/// 字句のデータ + 前後のトリビア
#[derive(Clone, Debug)]
pub(crate) struct FatToken {
    token: TokenData,
    leading: Vec<Trivia>,
    trailing: Vec<Trivia>,
}

impl FatToken {
    pub(crate) fn as_slim(&self) -> &TokenData {
        &self.token
    }

    pub(crate) fn into_slim(self) -> (Vec<Trivia>, TokenData, Vec<Trivia>) {
        (self.leading, self.token, self.trailing)
    }

    pub(crate) fn token(&self) -> Token {
        self.as_slim().token()
    }

    pub(crate) fn text(&self) -> &str {
        self.as_slim().text()
    }

    pub(crate) fn leading(&self) -> &[Trivia] {
        &self.leading
    }

    pub(crate) fn trailing(&self) -> &[Trivia] {
        &self.trailing
    }

    pub(crate) fn push_leading(&mut self, trivia: Trivia) {
        self.leading.push(trivia);
    }

    pub(crate) fn push_trailing(&mut self, trivia: Trivia) {
        self.trailing.push(trivia);
    }

    fn traverse_tokens<F: FnMut(&TokenData) -> bool>(&self, f: &mut F) -> bool {
        for trivia in self.leading() {
            if !f(trivia.as_token()) {
                return false;
            }
        }

        if !f(self.as_slim()) {
            return false;
        }

        for trivia in self.trailing() {
            if !f(trivia.as_token()) {
                return false;
            }
        }

        true
    }

    pub(crate) fn contains_eol(&self) -> bool {
        let mut ok = false;

        self.traverse_tokens(&mut |token| {
            if token.token() == Token::Eol {
                ok = true;
                return false;
            }
            true
        });

        ok
    }
}

impl From<TokenData> for FatToken {
    fn from(token: TokenData) -> Self {
        FatToken {
            token,
            leading: vec![],
            trailing: vec![],
        }
    }
}
