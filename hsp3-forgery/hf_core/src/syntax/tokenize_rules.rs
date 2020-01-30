use super::pun::PUN_TABLE;
use super::tokenize_context::TokenizeContext;
use super::*;

/// 文字が改行か？
fn char_is_eol(c: char) -> bool {
    c == '\r' || c == '\n'
}

/// 文字が改行ではない空白か？
fn char_is_space(c: char) -> bool {
    c == ' ' || c == '\t' || c == '　'
}

/// 文字が識別子の一部になるか？
fn char_is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 文字が識別子の先頭になるか？
fn char_is_ident_first(c: char) -> bool {
    char_is_ident(c) && !c.is_ascii_digit()
}

/// 文字が約物の先頭になるか？
fn char_is_pun_first(c: char) -> bool {
    "!\"#$%&'()-=^\\|@{}+*:,.<>/".contains(c)
}

/// 文字が解釈不能か？
fn char_is_other_first(c: char) -> bool {
    !char_is_eol(c)
        && !char_is_space(c)
        && c != ';'
        && !c.is_ascii_digit()
        && !char_is_ident_first(c)
        && !char_is_pun_first(c)
}

fn tokenize_eol(t: &mut TokenizeContext) -> bool {
    let mut ok = false;

    loop {
        while ok && char_is_space(t.next()) {
            t.bump();
        }

        if t.eat(";") || t.eat("//") {
            while !t.at_eof() && !char_is_eol(t.next()) {
                t.bump();
            }

            ok = true;
            continue;
        }

        if t.eat("\r\n") || t.eat("\n") {
            ok = true;
            continue;
        }

        break;
    }

    if ok {
        t.commit(Token::Eol);
        true
    } else {
        false
    }
}

fn tokenize_space(t: &mut TokenizeContext) -> bool {
    // 改行エスケープ
    if t.next() == '\\' && char_is_eol(t.nth(1)) {
        t.eat("\\");
        if !t.eat("\r\n") {
            t.eat("\n");
        }

        t.commit(Token::Space);
        return true;
    }

    let mut ok = false;

    while char_is_space(t.next()) {
        t.bump();
        ok = true;
    }

    if ok {
        t.commit(Token::Space);
        true
    } else {
        false
    }
}

fn tokenize_number(t: &mut TokenizeContext) -> bool {
    let mut ok = false;

    while t.next().is_ascii_digit() {
        t.bump();
        ok = true;
    }

    if ok {
        t.commit(Token::Digit);
        true
    } else {
        false
    }
}

fn tokenize_char_or_str_content(t: &mut TokenizeContext, quote: char) {
    while !t.at_eof() && !char_is_eol(t.next()) && t.next() != quote {
        // \ の直後が行末やファイル末尾のときはエスケープとみなさない。
        if t.eat("\\") && !t.at_eof() && !char_is_eol(t.next()) {
            t.commit(Token::StrEscape);
            continue;
        }

        let mut ok = false;

        while !t.at_eof() && !char_is_eol(t.next()) && t.next() != quote && t.next() != '\'' {
            t.bump();
            ok = true;
        }

        if ok {
            t.commit(Token::StrVerbatim);
            continue;
        }

        break;
    }
}

fn tokenize_char(t: &mut TokenizeContext) -> bool {
    if t.eat("'") {
        t.commit(Token::SingleQuote);

        tokenize_char_or_str_content(t, '\'');

        if t.eat("'") {
            t.commit(Token::SingleQuote);
        }

        return true;
    }

    false
}

fn tokenize_str(t: &mut TokenizeContext) -> bool {
    if t.eat("\"") {
        t.commit(Token::DoubleQuote);

        tokenize_char_or_str_content(t, '"');

        if t.eat("\"") {
            t.commit(Token::DoubleQuote);
        }

        return true;
    }

    false
}

fn tokenize_multiline_str(t: &mut TokenizeContext) -> bool {
    if t.eat("{\"") {
        t.commit(Token::LeftQuote);

        // FIXME: 各行の最初のタブ文字はスペース
        while !t.at_eof() && !t.is_followed_by("\"}") {
            t.bump();
        }
        t.commit(Token::StrVerbatim);

        if t.eat("\"}") {
            t.commit(Token::RightQuote);
        }

        return true;
    }

    false
}

fn tokenize_ident(t: &mut TokenizeContext) -> bool {
    if char_is_ident_first(t.next()) {
        while char_is_ident(t.next()) {
            t.bump();
        }

        let token = Token::parse_keyword(t.current_text()).unwrap_or(Token::Ident);

        t.commit(token);
        return true;
    }

    false
}

fn tokenize_pun(t: &mut TokenizeContext) -> bool {
    for &(token, pun_text) in PUN_TABLE {
        if t.eat(pun_text) {
            t.commit(token);
            return true;
        }
    }

    false
}

fn tokenize_other(t: &mut TokenizeContext) -> bool {
    if !t.at_eof() && char_is_other_first(t.next()) {
        while !t.at_eof() && char_is_other_first(t.next()) {
            t.bump();
        }

        t.commit(Token::Other);
        return true;
    }

    false
}

pub(crate) fn tokenize_all(t: &mut TokenizeContext) {
    while !t.at_eof() {
        let ok = tokenize_eol(t)
            || tokenize_space(t)
            || tokenize_number(t)
            || tokenize_char(t)
            || tokenize_str(t)
            || tokenize_multiline_str(t)
            || tokenize_ident(t)
            || tokenize_pun(t)
            || tokenize_other(t);

        assert!(ok, "無限ループ {}", t.current_index());
    }

    t.commit(Token::Eol);
}
