//! 約物 (punctuations)

use super::*;

pub(crate) static PUN_TABLE: &[(Token, &str)] = &[
    (Token::LeftParen, "("),
    (Token::RightParen, ")"),
    (Token::LeftAngle, "<"),
    (Token::RightAngle, ">"),
    (Token::LeftBrace, "{"),
    (Token::RightBrace, "}"),
    (Token::Colon, ":"),
    (Token::Comma, ","),
    (Token::Dot, "."),
    (Token::Equal, "="),
    (Token::Minus, "-"),
];
