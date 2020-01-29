use super::tokenize_context::TokenizeContext;
use super::*;
use std::collections::HashMap;
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

pub(crate) type TokensComponent = HashMap<Source, Vec<TokenData>>;

pub(crate) fn tokenize_sources(sources: &[(&Source, &Rc<String>)], tokenss: &mut TokensComponent) {
    for &(source, source_code) in sources {
        let tokens = tokenize(
            source.source_id,
            source.source_path.clone(),
            source_code.clone(),
        );
        tokenss.insert(source.clone(), tokens);
    }
}
