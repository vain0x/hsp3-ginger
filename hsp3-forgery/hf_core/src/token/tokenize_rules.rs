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

fn char_is_comment_first(c: char) -> bool {
    c == ';' || c == '/'
}

fn char_is_binary(c: char) -> bool {
    c == '0' || c == '1'
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
fn char_is_other_first(pp: bool, c: char) -> bool {
    if !pp && c == '#' {
        return true;
    }

    !char_is_eol(c)
        && !char_is_space(c)
        && !char_is_comment_first(c)
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

fn tokenize_space(pp: bool, t: &mut TokenizeContext) -> bool {
    // 改行エスケープ
    if pp && t.next() == '\\' && char_is_eol(t.nth(1)) {
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

fn tokenize_comment(t: &mut TokenizeContext) -> bool {
    if t.eat(";") || t.eat("//") {
        while !t.at_eof() && !char_is_eol(t.next()) {
            t.bump();
        }
        t.commit(Token::Comment);
        return true;
    }

    if t.eat("/*") {
        while !t.at_eof() && !t.eat("*/") {
            t.bump();
        }
        t.commit(Token::Comment);
        return true;
    }

    false
}

fn tokenize_binary(t: &mut TokenizeContext) {
    while char_is_binary(t.next()) {
        t.bump();
    }
    t.commit(Token::Binary);
}

fn tokenize_hex(t: &mut TokenizeContext) {
    while t.next().is_ascii_hexdigit() {
        t.bump();
    }
    t.commit(Token::Hex);
}

fn tokenize_number(pp: bool, t: &mut TokenizeContext) -> bool {
    if t.eat("0b") {
        t.commit(Token::ZeroB);
        tokenize_binary(t);
        return true;
    }

    if (pp && t.eat("%%")) || (!pp && t.eat("%")) {
        t.commit(Token::Percent);
        tokenize_binary(t);
        return true;
    }

    if t.eat("0x") {
        t.commit(Token::ZeroX);
        tokenize_hex(t);
        return true;
    }

    if t.eat("$") {
        t.commit(Token::Dollar);
        tokenize_hex(t);
        return true;
    }

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

fn tokenize_other(pp: bool, t: &mut TokenizeContext) -> bool {
    if !t.at_eof() && char_is_other_first(pp, t.next()) {
        while !t.at_eof() && char_is_other_first(pp, t.next()) {
            t.bump();
        }

        t.commit(Token::Other);
        return true;
    }

    false
}

fn tokenize_spaces_comments(pp: bool, t: &mut TokenizeContext) {
    while tokenize_space(pp, t) || tokenize_comment(t) {
        // Pass.
    }
}

/// プリプロセッサ命令における改行のエスケープや、
/// 複数行コメントや複数行文字列リテラルの中に改行を
fn tokenize_segment(t: &mut TokenizeContext) {
    // この時点で t は行頭に位置する。
    // 行頭のスペースやコメントを除去する。(複数行コメントの中に改行があっても1行とみなす。)
    tokenize_spaces_comments(false, t);

    let pp = if t.eat("#") {
        t.commit(Token::Hash);
        true
    } else {
        false
    };

    while !tokenize_eol(t) {
        let ok = tokenize_space(pp, t)
            || tokenize_comment(t)
            || tokenize_number(pp, t)
            || tokenize_char(t)
            || tokenize_str(t)
            || tokenize_multiline_str(t)
            || tokenize_ident(t)
            || tokenize_pun(t)
            || tokenize_other(pp, t);

        assert!(ok, "無限ループ {}", t.current_index());
    }
}

pub(crate) fn tokenize_all(t: &mut TokenizeContext) {
    while !t.at_eof() {
        tokenize_segment(t);
    }

    t.commit(Token::Eol);
}
