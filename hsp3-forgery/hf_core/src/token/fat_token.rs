use super::*;

pub(crate) struct FatToken {
    leading: Vec<Trivia>,
    body: TokenData,
    trailing: Vec<Trivia>,
}

impl FatToken {
    pub(crate) fn new(body: TokenData) -> Self {
        FatToken {
            leading: vec![],
            body,
            trailing: vec![],
        }
    }

    pub(crate) fn token(&self) -> Token {
        self.body.token()
    }

    pub(crate) fn text(&self) -> &str {
        self.body.text()
    }

    pub(crate) fn decompose(self) -> (Vec<Trivia>, TokenData, Vec<Trivia>) {
        (self.leading, self.body, self.trailing)
    }

    pub(crate) fn push_leading(&mut self, trivia: impl Into<Trivia>) {
        self.leading.push(trivia.into());
    }

    pub(crate) fn push_trailing(&mut self, trivia: impl Into<Trivia>) {
        self.trailing.push(trivia.into());
    }
}
