use super::*;
use crate::framework::*;
use crate::syntax::*;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct World {
    pub(crate) ids: IdProvider,
    pub(crate) workspaces: HashSet<Workspace>,
    pub(crate) source_files: SourceFileComponent,
    pub(crate) source_codes: SourceCodeComponent,
    pub(crate) tokenss: TokensComponent,
}

impl World {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

pub(crate) fn load_source_codes(w: &mut World) {
    crate::source::source_loader::load_sources(&w.source_files, &mut w.source_codes);
}

pub(crate) fn tokenize(w: &mut World) {
    let mut sources = vec![];
    for (&source_file_id, source_code) in &w.source_codes {
        let source = SyntaxSource::from_file(source_file_id, &w.source_files);
        sources.push((source, source_code));
    }

    crate::syntax::tokenize::tokenize_sources(&sources, &mut w.tokenss);
}
