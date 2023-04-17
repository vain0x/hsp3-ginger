//! 字句解析のルール

use super::tokenize_context::TokenizeContext;
use super::*;

type Tx = TokenizeContext;

/// 先読みの結果
#[derive(PartialEq, Eq)]
enum Lookahead {
    Eof,
    Cr,
    CrLf,
    Lf,
    EscapedCrLf,
    EscapedLf,
    Blank,
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
    Bad,
}

/// 何文字か先読みして、次の字句を決定する。
fn lookahead(tx: &mut Tx) -> Lookahead {
    match tx.next() {
        '\0' => Lookahead::Eof,
        '\r' => match tx.nth(1) {
            '\n' => Lookahead::CrLf,
            _ => Lookahead::Cr,
        },
        '\n' => Lookahead::Lf,
        ' ' | '\t' | '\u{3000}' => {
            // U+3000: 全角空白
            Lookahead::Blank
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
            '\r' => match tx.nth(2) {
                '\n' => Lookahead::EscapedCrLf,
                _ => Lookahead::Token(TokenKind::Backslash, 1),
            },
            '\n' => Lookahead::EscapedLf,
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
        'A'..='Z' | 'a'..='z' | '_' | '@' | '`' => Lookahead::Ident,
        c if c.is_whitespace() => {
            // 全角空白
            Lookahead::Blank
        }
        c if !c.is_control() && !c.is_ascii_punctuation() => {
            // 制御文字や記号を除いて、ほとんどの文字は識別子として認める。
            Lookahead::Ident
        }
        _ => Lookahead::Bad,
    }
}

/// 改行でない空白文字を読み飛ばす。
fn eat_blank(tx: &mut Tx) {
    loop {
        match tx.next() {
            ' ' | '\t' | '\u{3000}' => {
                tx.bump();
            }
            '\r' => match tx.nth(1) {
                '\n' => break,
                _ => tx.bump(),
            },
            '\n' => break,
            c if c.is_whitespace() => {
                tx.bump();
            }
            _ => break,
        }
    }
}

/// すべての空白を読み飛ばす。
fn eat_spaces(tx: &mut Tx) {
    loop {
        match tx.next() {
            ' ' | '\n' | '\r' | '\t' | '\u{3000}' => tx.bump(),
            c if c.is_whitespace() => tx.bump(),
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

        // 改行が見つからない場合は、いま最終行なので、ファイルの末尾まで読む。
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

/// エスケープシーケンスを含む引用符の中身を読み進める。`quote` が出てきたら終わり。
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

fn ident_to_kind(s: &str) -> TokenKind {
    match s {
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        _ => TokenKind::Ident,
    }
}

pub(crate) fn do_tokenize(tx: &mut Tx) {
    loop {
        match lookahead(tx) {
            Lookahead::Eof => break,
            Lookahead::Cr => {
                tx.bump();

                eat_blank(tx);
                tx.commit(TokenKind::Blank);
            }
            Lookahead::CrLf => {
                tx.bump_many(2);

                eat_spaces(tx);
                tx.commit(TokenKind::Newlines);
            }
            Lookahead::Lf => {
                tx.bump();

                eat_spaces(tx);
                tx.commit(TokenKind::Newlines);
            }
            Lookahead::EscapedCrLf => {
                tx.bump_many(3);

                eat_blank(tx);
                tx.commit(TokenKind::Blank);
            }
            Lookahead::EscapedLf => {
                tx.bump_many(2);

                eat_blank(tx);
                tx.commit(TokenKind::Blank);
            }
            Lookahead::Blank => {
                eat_blank(tx);
                tx.commit(TokenKind::Blank);
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

                while let Lookahead::Ident
                | Lookahead::ZeroB
                | Lookahead::ZeroX
                | Lookahead::Digit = lookahead(tx)
                {
                    tx.bump();
                }

                assert!(!tx.current_text().is_empty());
                let kind = ident_to_kind(tx.current_text());
                tx.commit(kind);
            }
            Lookahead::Token(kind, len) => {
                tx.bump_many(len);
                tx.commit(kind);
            }
            Lookahead::Bad => {
                tx.bump();

                while let Lookahead::Bad = lookahead(tx) {
                    tx.bump();
                }

                assert!(!tx.current_text().is_empty());
                tx.commit(TokenKind::Bad);
            }
        }
    }
}

pub(crate) fn tokenize(doc: DocId, text: RcStr) -> Vec<TokenData> {
    let mut tx = Tx::new(doc, text);
    do_tokenize(&mut tx);
    tx.finish()
}

#[cfg(test)]
mod tests {
    use super::{tokenize, TokenKind};

    fn tokenize_str_to_kinds(text: &str) -> Vec<TokenKind> {
        let mut kinds = {
            let tokens = tokenize(1, text.to_string().into());
            tokens
                .into_iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>()
        };

        // 末尾には必ず Eof がつく。個々の表明に含める必要はないので、ここで削除しておく。
        let eof = kinds.pop();
        assert_eq!(eof, Some(TokenKind::Eof));

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
            vec![TokenKind::Blank, TokenKind::Newlines]
        );
    }

    #[test]
    fn cr() {
        assert_eq!(tokenize_str_to_kinds("\r"), vec![TokenKind::Blank]);
        assert_eq!(tokenize_str_to_kinds("\r\r"), vec![TokenKind::Blank]);
    }

    #[test]
    fn comment_semi_with_eof() {
        assert_eq!(tokenize_str_to_kinds("; comment"), vec![TokenKind::Comment]);
    }

    #[test]
    fn comment_semi_with_eol() {
        assert_eq!(
            tokenize_str_to_kinds("; comment\n    "),
            vec![TokenKind::Comment, TokenKind::Newlines]
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
            vec![TokenKind::Comment, TokenKind::Newlines]
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
    fn ident_with_digits() {
        assert_eq!(tokenize_str_to_kinds("a0b0B0x0X42"), vec![TokenKind::Ident]);
    }

    #[test]
    fn ident_with_backticks() {
        assert_eq!(tokenize_str_to_kinds("`"), vec![TokenKind::Ident]);
    }

    #[test]
    fn ident_keyword() {
        assert_eq!(tokenize_str_to_kinds("if"), vec![TokenKind::If]);
        assert_eq!(tokenize_str_to_kinds("iff"), vec![TokenKind::Ident]);
    }

    // 数値の直後にある不要な文字は識別子トークンとみなすべきではないが、これで問題になるケースはたぶんなかったはずなので後回し。
    #[test]
    #[cfg(skip)]
    fn number_immediately_followed_by_ident() {
        assert_eq!(
            tokenize_str_to_kinds("1a"),
            vec![TokenKind::Number, TokenKind::Bad]
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

    #[test]
    fn escaped_eol() {
        assert_eq!(
            tokenize_str_to_kinds("#define \\\r\na\\\n42\\\n\nmes"),
            vec![
                TokenKind::Hash,
                TokenKind::Ident,
                TokenKind::Blank,
                TokenKind::Blank,
                TokenKind::Ident,
                TokenKind::Blank,
                TokenKind::Number,
                TokenKind::Blank,
                TokenKind::Newlines,
                TokenKind::Ident,
            ]
        )
    }

    // 未実装
    #[test]
    #[cfg(unimplemented)]
    fn macro_parameter() {
        assert_eq!(
            tokenize_str_to_kinds("#define id(%1) %1"),
            vec![
                TokenKind::Hash,
                TokenKind::Ident,
                TokenKind::Blank,
                TokenKind::Ident,
                TokenKind::LeftParen,
                TokenKind::Percent, // <- macro parameter であるべき
                TokenKind::Number,
                TokenKind::RightParen,
                TokenKind::Blank,
                TokenKind::Percent,
                TokenKind::Number,
            ]
        );
    }
}
