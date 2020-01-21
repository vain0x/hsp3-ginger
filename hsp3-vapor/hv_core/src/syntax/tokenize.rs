use super::tokenize_context::TokenizeContext;
use super::*;
use std::rc::Rc;

/// 字句解析を行う。
pub(crate) fn tokenize(source_code: Rc<String>) -> Box<[FatToken]> {
    let mut t = TokenizeContext::new(source_code);
    tokenize_rules::tokenize_all(&mut t);
    t.finish()
}

/// 字句解析を行い、各字句の開始位置のリストと、全体の長さを得る。
/// 位置や長さは UTF-16 基準。
pub(crate) fn tokenize_with_utf16_indices(source_code: Rc<String>) -> Box<[(Token, usize)]> {
    fn on_token(token: &TokenData, token_indices: &mut Vec<(Token, usize)>, index: &mut usize) {
        let start = *index;
        *index += token.text().encode_utf16().count();

        token_indices.push((token.token(), start));
    }

    fn on_fat_token(token: &FatToken, token_indices: &mut Vec<(Token, usize)>, index: &mut usize) {
        for trivia in token.leading() {
            on_token(trivia.as_token(), token_indices, index);
        }

        on_token(token.as_slim(), token_indices, index);

        for trivia in token.trailing() {
            on_token(trivia.as_token(), token_indices, index);
        }
    }

    let tokens = tokenize(source_code);

    let mut token_indices = vec![];
    let mut index = 0;

    for token in tokens.iter() {
        on_fat_token(token, &mut token_indices, &mut index);
    }

    token_indices.into_boxed_slice()
}
