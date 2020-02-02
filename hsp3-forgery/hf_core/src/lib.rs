pub(crate) mod analysis;
pub(crate) mod ast;
pub(crate) mod framework;
pub(crate) mod source;
pub(crate) mod syntax;
pub(crate) mod token;
pub(crate) mod workspace;
pub(crate) mod world;

pub(crate) use crate::workspace::Workspace;
pub(crate) use crate::world::World;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::*;
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
        let test_names = vec!["assign", "command", "exit_42", "syntax_error"];

        for name in test_names {
            let mut w = World::new();

            let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

            let (_workspace, source_file_id) =
                Workspace::new_with_file(source_path.clone(), &mut w.source_files, &mut w.ids);

            world::load_source_codes(&mut w);
            world::tokenize(&mut w);
            world::parse(&mut w);

            let source = SyntaxSource::from_file(source_file_id, &w.source_files);
            let ast_root = w.syntax_roots.get(&source).unwrap();

            write_snapshot(name, "ast.txt", &tests_dir, |out| {
                write!(out, "{:#?}\n", ast_root).unwrap();
            });
        }
    }

    #[test]
    fn completion_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let name = "assign";

        let mut w = World::new();

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let (_workspace, source_file_id) =
            Workspace::new_with_file(source_path, &mut w.source_files, &mut w.ids);

        world::load_source_codes(&mut w);
        world::tokenize(&mut w);
        world::parse(&mut w);

        let source = SyntaxSource::from_file(source_file_id, &w.source_files);
        let tokens = w.tokenss.get(&source).unwrap();
        let ast_root = ast::parse::parse(tokens);

        let position = Position {
            line: 4,
            character: 1,
        };
        let completion_items =
            crate::analysis::completion::get_completion_list(&ast_root, position);
        assert_eq!(completion_items.len(), 1);
    }

    #[test]
    fn signature_help_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let name = "command";

        let mut w = World::new();

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let (_workspace, source_file_id) =
            Workspace::new_with_file(source_path, &mut w.source_files, &mut w.ids);

        world::load_source_codes(&mut w);
        world::tokenize(&mut w);
        world::parse(&mut w);

        let source = SyntaxSource::from_file(source_file_id, &w.source_files);
        let ast_root = w.syntax_roots.get(&source).unwrap();

        // first
        let position = Position {
            line: 0,
            character: 7,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(0));

        // second
        let position = Position {
            line: 0,
            character: 13,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(1));

        // 範囲外
        let position = Position {
            line: 0,
            character: 1,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert!(signature_help_opt.is_none());
    }
}
