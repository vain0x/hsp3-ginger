//! 字句解析の規則

use super::pun::PUN_TABLE;
use super::tokenize_context::TokenizeContext;
use super::*;

/// 文字が改行か？
fn char_is_eol(c: char) -> bool {
    c == '\r' || c == '\n'
}

/// 文字が改行ではない空白か？
///
/// 全角空白も含む。
fn char_is_space(c: char) -> bool {
    c == ' ' || c == '\t' || c == '　'
}

/// 文字が識別子の一部になるか？
///
/// FIXME: 日本語の文字など
fn char_is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 文字が識別子の先頭になるか？
fn char_is_ident_first(c: char) -> bool {
    char_is_ident(c) && !c.is_ascii_digit()
}

/// 文字が約物の先頭になるか？
fn char_is_pun_first(c: char) -> bool {
    "()".contains(c)
}

/// 文字が解釈不能か？
fn char_is_other_first(c: char) -> bool {
    !char_is_eol(c)
        && !char_is_space(c)
        && c != '/'
        && !c.is_ascii_digit()
        && !char_is_ident_first(c)
        && !char_is_pun_first(c)
}

/// 改行を読み進める。
fn tokenize_eol(t: &mut TokenizeContext) {
    while t.eat("\r\n") {
        t.commit(Token::Eol);
    }

    while t.eat("\n") {
        t.commit(Token::Eol);
    }
}

/// 空白を読み進める。
fn tokenize_space(t: &mut TokenizeContext) {
    if char_is_space(t.next()) {
        while char_is_space(t.next()) {
            t.bump();
        }

        t.commit(Token::Space);
    }
}

/// コメントを読み進める。
fn tokenize_comment(t: &mut TokenizeContext) {
    if t.eat(";") || t.eat("//") {
        while !t.at_eof() && !char_is_eol(t.next()) {
            t.bump();
        }

        t.commit(Token::Comment)
    }
}

/// 数値を読み進める。
fn tokenize_number(t: &mut TokenizeContext) {
    if t.next().is_ascii_digit() {
        while t.next().is_ascii_digit() {
            t.bump();
        }

        t.commit(Token::Number);
    }
}

/// 識別子を読み進める。
fn tokenize_ident(t: &mut TokenizeContext) {
    if char_is_ident_first(t.next()) {
        while char_is_ident(t.next()) {
            t.bump();
        }

        let token = Token::parse_keyword(t.current_text()).unwrap_or(Token::Ident);

        t.commit(token);
    }
}

/// 約物を読み進める。
fn tokenize_pun(t: &mut TokenizeContext) {
    for &(token, pun_text) in PUN_TABLE {
        if t.eat(pun_text) {
            t.commit(token);
        }
    }
}

/// 解釈できない文字を読み進める。
fn tokenize_other(t: &mut TokenizeContext) {
    if !t.at_eof() && char_is_other_first(t.next()) {
        while !t.at_eof() && char_is_other_first(t.next()) {
            t.bump();
        }

        t.commit(Token::Other);
    }
}

/// 字句解析の規則をすべて適用する。
pub(crate) fn tokenize_all(t: &mut TokenizeContext) {
    while !t.at_eof() {
        let start_index = t.current_index();

        tokenize_eol(t);
        tokenize_space(t);
        tokenize_comment(t);
        tokenize_number(t);
        tokenize_ident(t);
        tokenize_pun(t);
        tokenize_other(t);

        assert_ne!(t.current_index(), start_index, "無限ループ");
    }
}
