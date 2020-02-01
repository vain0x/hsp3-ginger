use super::*;
use std::fs;
use std::rc::Rc;

pub(crate) fn load_sources(sources: &[SyntaxSource], source_codes: &mut SourceCodeComponent) {
    for source in sources {
        let source_code = match fs::read_to_string(source.source.source_path.as_ref()) {
            Ok(source_code) => source_code,
            Err(_) => continue,
        };
        source_codes.insert(source.clone(), Rc::new(source_code));
    }
}
