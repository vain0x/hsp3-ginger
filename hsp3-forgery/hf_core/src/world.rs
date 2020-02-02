use super::*;
use crate::ast::*;
use crate::framework::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct World {
    pub(crate) ids: IdProvider,
    pub(crate) workspaces: HashSet<Workspace>,
    pub(crate) source_files: SourceFileComponent,
}

impl World {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

pub(crate) fn load_source_codes(
    source_codes: &mut HashMap<SourceFileId, SourceCode>,
    w: &mut World,
) {
    for (&source_file_id, source_file) in &w.source_files {
        let source_code = match fs::read_to_string(source_file.source_path.as_ref()) {
            Ok(source_code) => source_code,
            Err(_) => continue,
        };
        source_codes.insert(source_file_id, Rc::new(source_code));
    }
}

pub(crate) fn tokenize(
    source_codes: &HashMap<SourceFileId, SourceCode>,
    tokenss: &mut HashMap<SyntaxSource, Vec<TokenData>>,
    w: &mut World,
) {
    let mut sources = vec![];
    for (&source_file_id, source_code) in source_codes {
        let source = SyntaxSource::from_file(source_file_id, &w.source_files);
        sources.push((source, source_code));
    }

    for (source, source_code) in sources {
        let tokens = crate::token::tokenize::tokenize(source.clone(), source_code.clone());
        tokenss.insert(source.clone(), tokens);
    }
}

pub(crate) fn parse(
    tokenss: &HashMap<SyntaxSource, Vec<TokenData>>,
    syntax_roots: &mut HashMap<SyntaxSource, ANodeData>,
    w: &mut World,
) {
    let mut sources = vec![];
    for (source, tokens) in tokenss {
        sources.push((source, tokens.as_slice()));
    }

    for (source, tokens) in sources {
        let root = crate::ast::parse::parse(tokens);
        syntax_roots.insert(source.clone(), root);
    }
}
