use super::tokenize_context::TokenizeContext;
use super::*;
use std::collections::HashMap;

pub(crate) fn tokenize(source: SyntaxSource, source_code: SourceCode) -> Vec<TokenData> {
    let mut t = TokenizeContext::new(source, source_code);
    tokenize_rules::tokenize_all(&mut t);
    t.finish()
}

pub(crate) type TokensComponent = HashMap<SyntaxSource, Vec<TokenData>>;

pub(crate) fn tokenize_sources(
    sources: &[(&SyntaxSource, &SourceCode)],
    tokenss: &mut TokensComponent,
) {
    for &(source, source_code) in sources {
        let tokens = tokenize(source.clone(), source_code.clone());
        tokenss.insert(source.clone(), tokens);
    }
}
