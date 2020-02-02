use super::tokenize_context::TokenizeContext;
use super::*;
use std::rc::Rc;

pub(crate) fn tokenize(source: SyntaxSource, source_code: Rc<SourceCode>) -> Vec<TokenData> {
    let mut t = TokenizeContext::new(source, source_code);
    tokenize_rules::tokenize_all(&mut t);
    t.finish()
}
