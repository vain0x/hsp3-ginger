use super::*;

type TokenIndex = usize;

pub(crate) struct ParseContext {
    tokens: Vec<TokenData>,
    errors: Vec<SyntaxError>,
}

impl ParseContext {
    pub(crate) fn new(mut tokens: Vec<TokenData>) -> Self {
        tokens.reverse();

        ParseContext {
            tokens,
            errors: vec![],
        }
    }

    pub(crate) fn current_index(&self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.next() == Token::Eof
    }

    fn nth(&self, offset: usize) -> Option<&TokenData> {
        assert!(offset < self.tokens.len());

        self.tokens.get(self.tokens.len() - offset - 1)
    }

    pub(crate) fn next(&self) -> Token {
        self.nth(0).map_or(Token::Eof, |token| token.token())
    }

    pub(crate) fn nth_data(&self, offset: usize) -> Option<&TokenData> {
        self.nth(offset)
    }

    pub(crate) fn next_data(&self) -> &TokenData {
        self.nth(0).unwrap()
    }

    pub(crate) fn bump(&mut self) -> TokenData {
        assert!(!self.tokens.is_empty());

        self.tokens.pop().unwrap()
    }

    pub(crate) fn eat(&mut self, token: Token) -> Option<TokenData> {
        if self.next() == token {
            Some(self.bump())
        } else {
            None
        }
    }

    pub(crate) fn eat_ident(&mut self, text: &str) -> Option<TokenData> {
        if self.next() == Token::Ident && self.next_data().text() == text {
            Some(self.bump())
        } else {
            None
        }
    }

    pub(crate) fn error(&mut self, msg: &str, token: &TokenData) {
        self.errors.push(SyntaxError {
            msg: msg.to_string(),
            location: token.location.clone(),
        })
    }

    pub(crate) fn error_next(&mut self, msg: &str) {
        let location = self.nth(0).unwrap().location.clone();

        self.errors.push(SyntaxError {
            msg: msg.to_string(),
            location,
        })
    }

    pub(crate) fn finish(mut self) -> (TokenData, Vec<SyntaxError>) {
        assert_eq!(self.tokens.len(), 1);

        let eof_token = self.bump();
        let errors = std::mem::replace(&mut self.errors, vec![]);

        (eof_token, errors)
    }
}
