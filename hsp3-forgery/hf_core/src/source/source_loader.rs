use super::*;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

pub(crate) fn load_sources(
    source_files: &SourceFileComponent,
    source_codes: &mut SourceCodeComponent,
) {
    for (&source_file_id, source_file) in source_files {
        let source_code = match fs::read_to_string(source_file.source_path.as_ref()) {
            Ok(source_code) => source_code,
            Err(_) => continue,
        };
        source_codes.insert(source_file_id, Rc::new(source_code));
    }
}
