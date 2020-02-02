use super::*;
use crate::ast::*;
use crate::framework::*;
use crate::token::*;
use std::collections::{HashMap, HashSet};

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

    crate::token::tokenize::tokenize_sources(&sources, &mut w.tokenss);
}

pub(crate) fn parse(syntax_roots: &mut HashMap<SyntaxSource, ANodeData>, w: &mut World) {
    let mut sources = vec![];
    for (source, tokens) in &w.tokenss {
        sources.push((source.clone(), tokens.as_slice()));
    }

    for (source, tokens) in sources {
        let root = crate::ast::parse::parse(tokens);
        syntax_roots.insert(source.clone(), root);
    }
}
