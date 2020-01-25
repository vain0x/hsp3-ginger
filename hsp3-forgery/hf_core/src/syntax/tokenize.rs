use super::tokenize_context::TokenizeContext;
use super::*;
use std::rc::Rc;

pub(crate) fn tokenize(source_id: usize, source_code: Rc<String>) -> Vec<TokenData> {
    let mut t = TokenizeContext::new(source_id, source_code);
    tokenize_rules::tokenize_all(&mut t);
    t.finish()
}
