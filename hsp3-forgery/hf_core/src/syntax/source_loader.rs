use super::*;
use std::fs;
use std::rc::Rc;

pub(crate) fn load_sources(
    sources: &[SyntaxSource],
    source_files: &SourceFileComponent,
    source_codes: &mut SourceCodeComponent,
) {
    for source in sources {
        let source_path = match source_files
            .get(&source.source_file_id)
            .map(|source_file| source_file.source_path.as_ref())
        {
            None => continue,
            Some(source_path) => source_path,
        };

        let source_code = match fs::read_to_string(source_path) {
            Ok(source_code) => source_code,
            Err(_) => continue,
        };
        source_codes.insert(source.clone(), Rc::new(source_code));
    }
}
