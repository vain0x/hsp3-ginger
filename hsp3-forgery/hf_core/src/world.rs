use crate::ast::*;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

pub(crate) fn load_source_codes(
    source_files: impl Iterator<Item = SourceFile>,
    source_codes: &mut HashMap<SourceFile, Rc<SourceCode>>,
) {
    for source_file in source_files {
        let source_code = match fs::read_to_string(source_file.source_path.as_ref()) {
            Ok(source_code) => source_code,
            Err(_) => continue,
        };
        source_codes.insert(source_file, Rc::new(source_code));
    }
}

pub(crate) fn tokenize(
    source_codes: &HashMap<SourceFile, Rc<SourceCode>>,
    tokenss: &mut HashMap<SyntaxSource, Vec<TokenData>>,
) {
    let mut sources = vec![];
    for (source_file, source_code) in source_codes {
        let source = SyntaxSource::from_file(source_file.clone());
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
