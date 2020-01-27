use super::tokenize_context::TokenizeContext;
use super::*;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) fn tokenize(
    source_id: usize,
    source_path: Rc<PathBuf>,
    source_code: Rc<String>,
) -> Vec<TokenData> {
    let mut t = TokenizeContext::new(source_id, source_path, source_code);
    tokenize_rules::tokenize_all(&mut t);
    t.finish()
}
