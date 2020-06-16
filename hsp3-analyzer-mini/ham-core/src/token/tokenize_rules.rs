//! 字句解析のルール

#![allow(unused)]

use super::{tokenize_context::TokenizeContext, TokenData, TokenKind};
use crate::{analysis::ADoc, utils::rc_str::RcStr};

type Tx = TokenizeContext;

#[derive(PartialEq, Eq)]
enum Lookahead {
    Eof,
    Eol,
    Space,
    Semi,
    SlashSlash,
    SlashStar,
    ZeroB,
    ZeroX,
    Dollar,
    Digit,
    SingleQuote,
    DoubleQuote,
    HereDocument,
    Ident,
    Token(TokenKind, usize),
    Other,
}

/// 何文字か先読みして、次の字句を推測する。
fn lookahead(tx: &mut Tx) -> Lookahead {
    match tx.next() {
        '\0' => Lookahead::Eof,
        '\n' | '\r' => Lookahead::Eol,
        ' ' | '\t' | '\u{3000}' => {
            // U+3000: 全角空白
            Lookahead::Space
        }
        '0' => match tx.nth(1) {
            'b' | 'B' => Lookahead::ZeroB,
            'x' | 'X' => Lookahead::ZeroX,
            _ => Lookahead::Digit,
        },
        '$' => Lookahead::Dollar,
        '\'' => Lookahead::SingleQuote,
        '"' => Lookahead::DoubleQuote,
        ';' => Lookahead::Semi,
        '(' => Lookahead::Token(TokenKind::LeftParen, 1),
        ')' => Lookahead::Token(TokenKind::RightParen, 1),
        '{' => match tx.nth(1) {
            '"' => Lookahead::HereDocument,
            _ => Lookahead::Token(TokenKind::LeftBrace, 1),
        },
        '}' => Lookahead::Token(TokenKind::RightBrace, 1),
        '<' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::LeftEqual, 2),
            '<' => Lookahead::Token(TokenKind::LeftShift, 2),
            _ => Lookahead::Token(TokenKind::LeftAngle, 1),
        },
        '>' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::RightEqual, 2),
            '>' => Lookahead::Token(TokenKind::RightShift, 2),
            _ => Lookahead::Token(TokenKind::RightAngle, 1),
        },
        '&' => match tx.nth(1) {
            '&' => Lookahead::Token(TokenKind::AndAnd, 2),
            '=' => Lookahead::Token(TokenKind::AndEqual, 2),
            _ => Lookahead::Token(TokenKind::And, 1),
        },
        '\\' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::BackslashEqual, 2),
            _ => Lookahead::Token(TokenKind::Backslash, 1),
        },
        '!' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::BangEqual, 2),
            _ => Lookahead::Token(TokenKind::Bang, 1),
        },
        ':' => Lookahead::Token(TokenKind::Colon, 1),
        ',' => Lookahead::Token(TokenKind::Comma, 1),
        '.' => Lookahead::Token(TokenKind::Dot, 1),
        '=' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::EqualEqual, 2),
            _ => Lookahead::Token(TokenKind::Equal, 1),
        },
        '#' => Lookahead::Token(TokenKind::Hash, 1),
        '^' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::HatEqual, 2),
            _ => Lookahead::Token(TokenKind::Hat, 1),
        },
        '-' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::MinusEqual, 2),
            '-' => Lookahead::Token(TokenKind::MinusMinus, 2),
            '>' => Lookahead::Token(TokenKind::SlimArrow, 2),
            _ => Lookahead::Token(TokenKind::Minus, 1),
        },
        '%' => Lookahead::Token(TokenKind::Percent, 1),
        '|' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::PipeEqual, 2),
            '|' => Lookahead::Token(TokenKind::PipePipe, 2),
            _ => Lookahead::Token(TokenKind::Pipe, 1),
        },
        '+' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::PlusEqual, 2),
            '+' => Lookahead::Token(TokenKind::PlusPlus, 2),
            _ => Lookahead::Token(TokenKind::Plus, 1),
        },
        '/' => match tx.nth(1) {
            '/' => Lookahead::SlashSlash,
            '*' => Lookahead::SlashStar,
            '=' => Lookahead::Token(TokenKind::SlashEqual, 2),
            _ => Lookahead::Token(TokenKind::Slash, 1),
        },
        '*' => match tx.nth(1) {
            '=' => Lookahead::Token(TokenKind::StarEqual, 2),
            _ => Lookahead::Token(TokenKind::Star, 1),
        },
        '1'..='9' => Lookahead::Digit,
        'A'..='Z' | 'a'..='z' | '_' | '@' => Lookahead::Ident,
        c if c.is_whitespace() => {
            // 全角空白
            Lookahead::Space
        }
        c if !c.is_control() && !c.is_ascii_punctuation() => {
            // 制御文字や、上記に列挙されていない記号を除いて、ほとんどの文字は識別子として認める。
            Lookahead::Ident
        }
        _ => Lookahead::Other,
    }
}

fn eat_spaces(tx: &mut Tx) {
    loop {
        match tx.next() {
            ' ' | '\t' | '\n' | '\r' | '\u{3000}' => {
                tx.bump();
            }
            c if c.is_whitespace() => {
                tx.bump();
            }
            _ => break,
        }
    }
}

/// 行末まで読み飛ばす。改行自体は読まない。
fn eat_line(tx: &mut Tx) {
    match tx.find("\n") {
        Some(mut len) => {
            // CRLF の LF が見つかったときは CR の前に戻る。
            if len >= 1 && tx.nth_byte(len - 1) == b'\r' {
                len -= 1;
            }

            tx.bump_many(len)
        }
        None => tx.bump_all(),
    }
}

fn eat_binary_digits(tx: &mut Tx) {
    while let '0' | '1' = tx.next() {
        tx.bump();
    }
}

fn eat_hex_digits(tx: &mut Tx) {
    while tx.next().is_ascii_hexdigit() {
        tx.bump();
    }
}

fn eat_digits(tx: &mut Tx) {
    while tx.next().is_ascii_digit() {
        tx.bump();
    }
}

/// 10進数の数字の直後にある、小数部や指数部を字句解析する。
fn tokenize_digit_suffix(tx: &mut TokenizeContext) {
    // 小数部
    if tx.eat(".") {
        eat_digits(tx);
    }

    // 指数部
    if let 'e' | 'E' = tx.next() {
        tx.bump();

        if let '+' | '-' = tx.next() {
            tx.bump();
        }

        eat_digits(tx);
    }
}

fn eat_escaped_text(quote: char, tx: &mut Tx) {
    loop {
        match tx.next() {
            '\0' | '\n' | '\r' => break,
            '\\' => {
                tx.bump();
                tx.bump();
            }
            c if c == quote => break,
            _ => tx.bump(),
        }
    }
}

pub(crate) fn do_tokenize(tx: &mut Tx) {
    loop {
        match lookahead(tx) {
            Lookahead::Eof => {
                tx.commit(TokenKind::Eol);
                break;
            }
            Lookahead::Eol => {
                tx.commit(TokenKind::Eol);

                eat_spaces(tx);
                tx.commit(TokenKind::Space);
            }
            Lookahead::Space => {
                eat_spaces(tx);
                tx.commit(TokenKind::Space);
            }
            Lookahead::Semi => {
                tx.bump();
                eat_line(tx);

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Comment);
            }
            Lookahead::SlashSlash => {
                tx.bump_many(2);
                eat_line(tx);

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Comment);
            }
            Lookahead::SlashStar => {
                tx.bump_many(2);

                match tx.find("*/") {
                    Some(len) => tx.bump_many(len + 2),
                    None => tx.bump_all(),
                }

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Comment);
            }
            Lookahead::ZeroB => {
                tx.bump_many(2);

                eat_binary_digits(tx);
                tx.commit(TokenKind::Number);
            }
            Lookahead::ZeroX => {
                tx.bump_many(2);

                eat_hex_digits(tx);
                tx.commit(TokenKind::Number);
            }
            Lookahead::Dollar => {
                tx.bump();

                eat_hex_digits(tx);
                tx.commit(TokenKind::Number);
            }
            Lookahead::Digit => {
                eat_digits(tx);
                assert!(!tx.current_text().is_empty());

                tokenize_digit_suffix(tx);
                tx.commit(TokenKind::Number);
            }
            Lookahead::SingleQuote => {
                tx.bump();

                eat_escaped_text('\'', tx);
                tx.eat("\'");

                tx.commit(TokenKind::Char);
            }
            Lookahead::DoubleQuote => {
                tx.bump();

                eat_escaped_text('"', tx);
                tx.eat("\"");

                tx.commit(TokenKind::Str);
            }
            Lookahead::HereDocument => {
                tx.bump_many(2);

                match tx.find("\"}") {
                    Some(len) => tx.bump_many(len + 2),
                    None => tx.bump_all(),
                }

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Str);
            }
            Lookahead::Ident => {
                tx.bump();

                while let Lookahead::Ident | Lookahead::Digit = lookahead(tx) {
                    tx.bump();
                }

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Ident);
            }
            Lookahead::Token(kind, len) => {
                tx.bump_many(len);
                tx.commit(kind);
            }
            Lookahead::Other => {
                tx.bump();

                while let Lookahead::Other = lookahead(tx) {
                    tx.bump();
                }

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Other);
            }
        }
    }
}

pub(crate) fn tokenize(doc: ADoc, text: RcStr) -> Vec<TokenData> {
    let mut tx = Tx::new(doc, text);
    do_tokenize(&mut tx);
    tx.finish()
}

#[cfg(test)]
mod tests {
    use super::{tokenize, ADoc, TokenKind};

    fn tokenize_str_to_kinds(text: &str) -> Vec<TokenKind> {
        let mut kinds = {
            let tokens = tokenize(ADoc::new(1), text.to_string().into());
            tokens
                .into_iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>()
        };

        // 末尾には必ず Eol, Eof がつく。個々の表明に含める必要はないので、ここで削除しておく。
        let eof = kinds.pop();
        assert_eq!(eof, Some(TokenKind::Eof));

        let eol = kinds.pop();
        assert_eq!(eol, Some(TokenKind::Eol));

        kinds
    }

    #[test]
    fn empty() {
        assert_eq!(tokenize_str_to_kinds(""), vec![]);
    }

    #[test]
    fn space() {
        assert_eq!(
            tokenize_str_to_kinds(" \r\n\t\u{3000}　"),
            vec![TokenKind::Space]
        );
    }

    #[test]
    fn comment_semi_with_eof() {
        assert_eq!(tokenize_str_to_kinds("; comment"), vec![TokenKind::Comment]);
    }

    #[test]
    fn comment_semi_with_eol() {
        assert_eq!(
            tokenize_str_to_kinds("; comment\n    "),
            vec![TokenKind::Comment, TokenKind::Eol, TokenKind::Space]
        );
    }

    #[test]
    fn comment_slash_with_eof() {
        assert_eq!(tokenize_str_to_kinds("////"), vec![TokenKind::Comment]);
    }

    #[test]
    fn comment_slash_with_eol() {
        assert_eq!(
            tokenize_str_to_kinds("// 🐧\n"),
            vec![TokenKind::Comment, TokenKind::Eol, TokenKind::Space]
        );
    }

    #[test]
    fn comment_multiline() {
        assert_eq!(
            tokenize_str_to_kinds("/* 🐧\n*/*/"),
            vec![TokenKind::Comment, TokenKind::Star, TokenKind::Slash]
        );
    }

    #[test]
    fn number_zero() {
        assert_eq!(tokenize_str_to_kinds("0"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_digits() {
        assert_eq!(tokenize_str_to_kinds("1234567890"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_with_fraction() {
        assert_eq!(tokenize_str_to_kinds("3.14"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_with_exp() {
        assert_eq!(tokenize_str_to_kinds("1e9"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_with_exp_plus() {
        assert_eq!(tokenize_str_to_kinds("1e+9"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_with_exp_minus() {
        assert_eq!(tokenize_str_to_kinds("1e-9"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_float() {
        assert_eq!(tokenize_str_to_kinds("6.02e23"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_zero_b() {
        assert_eq!(tokenize_str_to_kinds("0b0101"), vec![TokenKind::Number]);
        assert_eq!(tokenize_str_to_kinds("0B1111"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_percent() {
        // FIXME: プリプロセッサ行かどうかで % の解釈が変わる。
        assert_eq!(
            tokenize_str_to_kinds("%0101"),
            vec![TokenKind::Percent, TokenKind::Number]
        );
    }

    #[test]
    fn number_zero_x() {
        assert_eq!(tokenize_str_to_kinds("0xabcdef16"), vec![TokenKind::Number]);
        assert_eq!(tokenize_str_to_kinds("0XABCDEF16"), vec![TokenKind::Number]);
    }

    #[test]
    fn number_dollar() {
        assert_eq!(tokenize_str_to_kinds("$deadbeef"), vec![TokenKind::Number]);
    }

    #[test]
    fn char() {
        assert_eq!(tokenize_str_to_kinds("'a'"), vec![TokenKind::Char]);
        assert_eq!(tokenize_str_to_kinds("'\"'"), vec![TokenKind::Char]);
        assert_eq!(tokenize_str_to_kinds("'\\''"), vec![TokenKind::Char]);
        assert_eq!(tokenize_str_to_kinds("'\\\\'"), vec![TokenKind::Char]);
        assert_eq!(tokenize_str_to_kinds("'你'"), vec![TokenKind::Char]);
        assert_eq!(tokenize_str_to_kinds("'🐧'"), vec![TokenKind::Char]);
    }

    #[test]
    fn str() {
        assert_eq!(
            tokenize_str_to_kinds("\"hello, world!\""),
            vec![TokenKind::Str]
        );
        assert_eq!(tokenize_str_to_kinds("\"\""), vec![TokenKind::Str]);
        assert_eq!(
            tokenize_str_to_kinds(r#"" sq' dq\" lf\n backslash\\ ""#),
            vec![TokenKind::Str]
        );
        assert_eq!(tokenize_str_to_kinds("\"你好☺\""), vec![TokenKind::Str]);
    }

    #[test]
    fn here_document() {
        assert_eq!(
            tokenize_str_to_kinds(
                r#"{"
                    🐧 "you can write anything here!"
                "}"#
            ),
            vec![TokenKind::Str]
        )
    }

    #[test]
    fn ident_ascii() {
        assert_eq!(
            tokenize_str_to_kinds("lower_UPPER_42"),
            vec![TokenKind::Ident]
        );
    }

    #[test]
    fn ident_with_at_sign() {
        assert_eq!(tokenize_str_to_kinds("stat@hsp3"), vec![TokenKind::Ident]);
    }

    #[test]
    fn ident_non_ascii() {
        assert_eq!(
            tokenize_str_to_kinds("こんにちはhello你好"),
            vec![TokenKind::Ident]
        );
    }

    #[test]
    fn punctuations() {
        assert_eq!(
            tokenize_str_to_kinds("(){}=->"),
            vec![
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Equal,
                TokenKind::SlimArrow,
            ]
        )
    }
}
