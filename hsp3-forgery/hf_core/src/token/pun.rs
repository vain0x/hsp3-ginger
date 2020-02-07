//! 約物 (punctuation)
//!
//! 文法上の意味を持つ記号のこと。

use super::*;

pub(crate) static PUN_TABLE: &[(Token, &str)] = &[
    (Token::LeftParen, "("),
    (Token::RightParen, ")"),
    (Token::LeftBrace, "{"),
    (Token::RightBrace, "}"),
    (Token::AndAnd, "&&"),
    (Token::And, "&"),
    (Token::AtSign, "@"),
    (Token::Backslash, "\\"),
    (Token::BangEqual, "!="),
    (Token::Bang, "!"),
    (Token::Colon, ":"),
    (Token::Comma, ","),
    (Token::Dollar, "$"),
    (Token::Dot, "."),
    (Token::EqualEqual, "=="),
    (Token::Equal, "="),
    (Token::Hat, "^"),
    (Token::LeftShift, "<<"),
    (Token::LeftEqual, "<="),
    (Token::Minus, "-"),
    (Token::Percent, "%"),
    (Token::PipePipe, "||"),
    (Token::Pipe, "|"),
    (Token::Plus, "+"),
    (Token::RightEqual, ">="),
    (Token::RightShift, ">>"),
    (Token::Slash, "/"),
    (Token::Star, "*"),
    (Token::LeftAngle, "<"),
    (Token::RightAngle, ">"),
];

impl Token {
    pub(crate) fn parse_pun(text: &str) -> Option<Token> {
        PUN_TABLE
            .iter()
            .filter_map(
                |&(token, pun_text)| {
                    if text == pun_text {
                        Some(token)
                    } else {
                        None
                    }
                },
            )
            .next()
    }
}
