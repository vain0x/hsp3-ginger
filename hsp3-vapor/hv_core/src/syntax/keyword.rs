use super::*;

pub(crate) static KEYWORD_TABLE: &[(Token, &str)] = &[
    (Token::If, "if"),
    (Token::Else, "else"),
    (Token::Repeat, "repeat"),
    (Token::Loop, "loop"),
    (Token::Break, "break"),
    (Token::Continue, "continue"),
    (Token::Return, "return"),
];

impl Token {
    pub(crate) fn is_control_keyword(self) -> bool {
        self == Token::If
            || self == Token::Else
            || self == Token::Repeat
            || self == Token::Loop
            || self == Token::Break
            || self == Token::Continue
            || self == Token::Return
    }

    pub(crate) fn is_keyword(self) -> bool {
        self.is_control_keyword()
    }

    pub(crate) fn parse_keyword(text: &str) -> Option<Token> {
        KEYWORD_TABLE
            .iter()
            .filter_map(|&(keyword, keyword_text)| {
                if text == keyword_text {
                    Some(keyword)
                } else {
                    None
                }
            })
            .next()
    }
}
