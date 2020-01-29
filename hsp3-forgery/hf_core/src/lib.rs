pub(crate) mod analysis;
pub(crate) mod ast;
pub(crate) mod id_provider;
pub(crate) mod kir;
pub(crate) mod syntax;
pub(crate) mod workspace;

pub(crate) use id_provider::IdProvider;
pub(crate) use workspace::{SourceComponent, Workspace};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::*;
    use std::fs;
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    fn write_snapshot(name: &str, suffix: &str, tests_dir: &Path, f: impl Fn(&mut Vec<u8>)) {
        let mut out = vec![];
        f(&mut out);

        let file_path = tests_dir.join(format!("{}/{}_snapshot_{}", name, name, suffix));
        fs::write(&file_path, out).unwrap();
    }

    fn snapshot_test(name: &str, tests_dir: &Path) {}

    #[test]
    fn snapshot_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let mut ids = IdProvider::new();
        let mut sources = SourceComponent::default();

        let test_names = vec!["assign", "command", "exit_42"];

        for name in test_names {
            let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

            let (workspace, source) =
                Workspace::new_with_file(source_path.clone(), &mut sources, &mut ids);
            let mut source_codes = SourceCodeComponent::default();

            syntax::source_loader::load_sources(
                &sources.get(&workspace).unwrap_or(&vec![]),
                &mut source_codes,
            );

            let source_code = source_codes
                .get(&source)
                .map(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let tokens =
                syntax::tokenize::tokenize(source.source_id, source_path, Rc::new(source_code));
            let ast_root = ast::parse::parse(tokens);

            write_snapshot(name, "ast.txt", &tests_dir, |out| {
                write!(out, "{:#?}\n", ast_root).unwrap();
            });

            let kir_root = kir::gen::gen(ast_root);
            write_snapshot(name, "kir.txt", &tests_dir, |out| {
                write!(out, "{:#?}\n", kir_root).unwrap();
            });
        }
    }

    #[test]
    fn completion_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let name = "assign";

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

        use crate::analysis::completion::*;
        let id_provider = IdProvider::new();
        let mut project = Project::new();

        load_source(source_path.clone(), &id_provider, &mut project.sources).unwrap();
        let source_id = project.sources.path_to_id(source_path.as_ref()).unwrap();

        let position = syntax::Position {
            line: 4,
            character: 1,
        };
        let completion_items =
            crate::analysis::completion::get_completion_list(source_id, position, &mut project);
        assert_eq!(completion_items.len(), 1);
    }

    #[test]
    fn signature_help_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let name = "command";

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

        use crate::analysis::completion::*;
        let id_provider = IdProvider::new();
        let mut project = Project::new();

        load_source(source_path.clone(), &id_provider, &mut project.sources).unwrap();
        let source_id = project.sources.path_to_id(source_path.as_ref()).unwrap();

        // first
        let position = syntax::Position {
            line: 0,
            character: 7,
        };
        let signature_help_opt =
            crate::analysis::completion::signature_help(source_id, position, &mut project);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(0));

        // second
        let position = syntax::Position {
            line: 0,
            character: 13,
        };
        let signature_help_opt =
            crate::analysis::completion::signature_help(source_id, position, &mut project);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(1));

        // 範囲外
        let position = syntax::Position {
            line: 0,
            character: 1,
        };
        let signature_help_opt =
            crate::analysis::completion::signature_help(source_id, position, &mut project);
        assert!(signature_help_opt.is_none());
    }
}
