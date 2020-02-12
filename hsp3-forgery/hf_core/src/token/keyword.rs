//! キーワード
//!
//! 厳密には `mes` や `int` など大量のキーワードがあるが、
//! 列挙するのはめんどうなため、いまのところ重要なものだけをキーワードに指定している。

use super::*;

pub(crate) static KEYWORD_TABLE: &[(Token, &str)] = &[
    (Token::Break, "break"),
    (Token::Cnt, "cnt"),
    (Token::Continue, "continue"),
    (Token::Else, "else"),
    (Token::End, "end"),
    (Token::Foreach, "foreach"),
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
    /// jump modifier とは:
    ///     `button gosub "OK", *l_ok` のように、
    ///     命令の直後に書かれている goto/gosub のこと。
    pub(crate) fn is_jump_modifier(self) -> bool {
        match self {
            Token::Gosub | Token::Goto => true,
            _ => false,
        }
    }

    /// control keyword とは:
    ///     命令を上から下に実行していくのとは異なる動作をさせる命令。
    ///     `goto` や `stop` のように、直後の命令に実行が進まないものや、
    ///     `repeat` のように別の命令から飛んでくる先となる命令。
    pub(crate) fn is_control_keyword(self) -> bool {
        match self {
            Token::Break
            | Token::Continue
            | Token::Else
            | Token::End
            | Token::Foreach
            | Token::Gosub
            | Token::Goto
            | Token::If
            | Token::Loop
            | Token::Repeat
            | Token::Return
            | Token::Stop => true,
            _ => false,
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
