use super::*;

pub(crate) static KEYWORD_TABLE: &[(Token, &str)] = &[
    (Token::Break, "break"),
    (Token::Cnt, "cnt"),
    (Token::Continue, "continue"),
    (Token::Else, "else"),
    (Token::End, "end"),
    (Token::Gosub, "gosub"),
    (Token::Goto, "goto"),
    (Token::If, "if"),
    (Token::Loop, "loop"),
    (Token::Refdval, "refdval"),
    (Token::Refstr, "refstr"),
    (Token::Repeat, "repeat"),
    (Token::Return, "return"),
    (Token::Stat, "stat"),
    (Token::Stop, "stop"),
    (Token::Strsize, "strsize"),
];

impl Token {
    pub(crate) fn is_control_keyword(self) -> bool {
        self == Token::Break
            || self == Token::Continue
            || self == Token::Else
            || self == Token::End
            || self == Token::Gosub
            || self == Token::Goto
            || self == Token::If
            || self == Token::Loop
            || self == Token::Repeat
            || self == Token::Return
            || self == Token::Stop
    }

    pub(crate) fn is_system_var_keyword(self) -> bool {
        self == Token::Cnt
            || self == Token::Refdval
            || self == Token::Refstr
            || self == Token::Stat
            || self == Token::Strsize
    }

    pub(crate) fn is_keyword(self) -> bool {
        self.is_control_keyword() || self.is_system_var_keyword()
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
