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
    pub(crate) fn is_jump_modifier(self) -> bool {
        match self {
            Token::Gosub | Token::Goto => true,
            _ => false,
        }
    }

    pub(crate) fn is_jump_keyword(self) -> bool {
        match self {
            Token::Break
            | Token::Continue
            | Token::End
            | Token::Gosub
            | Token::Goto
            | Token::Loop
            | Token::Repeat
            | Token::Return
            | Token::Stop => true,
            _ => false,
        }
    }

    pub(crate) fn is_control_keyword(self) -> bool {
        match self {
            Token::Else | Token::If => true,
            _ => self.is_jump_keyword(),
        }
    }

    pub(crate) fn is_system_var_keyword(self) -> bool {
        match self {
            Token::Cnt | Token::Refdval | Token::Refstr | Token::Stat | Token::Strsize => true,
            _ => false,
        }
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
