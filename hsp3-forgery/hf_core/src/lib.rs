pub(crate) mod analysis;
pub mod api;
pub(crate) mod ast;
pub(crate) mod framework;
pub(crate) mod parse;
pub(crate) mod source;
pub(crate) mod syntax;
pub(crate) mod token;
pub(crate) mod workspace;
pub(crate) mod world;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::*;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    fn write_snapshot(name: &str, suffix: &str, tests_dir: &Path, f: impl Fn(&mut Vec<u8>)) {
        let mut out = vec![];
        f(&mut out);

        let file_path = tests_dir.join(format!("{}/{}_snapshot_{}", name, name, suffix));
        fs::write(&file_path, out).unwrap();
    }

    #[test]
    fn snapshot_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let test_names = vec![
            "api_signature_help",
            "assign",
            "command",
            "exit_42",
            "syntax_error",
            "syntax_pp",
        ];

        for name in test_names {
            let mut source_files = HashSet::new();
            let mut source_codes = HashMap::new();
            let mut tokenss = HashMap::new();

            let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
            let source_file = SourceFile { source_path };
            source_files.insert(source_file.clone());

            world::load_source_codes(source_files.iter().cloned(), &mut source_codes);
            world::tokenize(&source_codes, &mut tokenss);

            let source = TokenSource::from_file(source_file);
            let tokens = tokenss.get(&source).unwrap();
            let root = crate::parse::parse_tokens(tokens);

            write_snapshot(name, "syntax.txt", &tests_dir, |out| {
                write!(out, "{:#?}", root).unwrap();
            })
        }
    }

    #[test]
    fn completion_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let name = "assign";

        let mut source_files = HashSet::new();
        let mut source_codes = HashMap::new();
        let mut tokenss = HashMap::new();

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let source_file = SourceFile { source_path };
        source_files.insert(source_file.clone());

        world::load_source_codes(source_files.iter().cloned(), &mut source_codes);
        world::tokenize(&source_codes, &mut tokenss);

        let source = TokenSource::from_file(source_file);
        let tokens = tokenss.get(&source).unwrap();
        let syntax_root = crate::parse::parse_tokens(tokens);

        let position = Position {
            line: 4,
            character: 1,
        };
        let completion_items =
            crate::analysis::completion::get_completion_list(syntax_root, position);
        assert_eq!(completion_items.len(), 1);
    }
}
