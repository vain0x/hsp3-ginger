pub(crate) mod analysis;
pub(crate) mod ast;
pub(crate) mod kir;
pub(crate) mod syntax;

#[cfg(test)]
mod tests {
    use super::*;
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

    fn snapshot_test(name: &str, tests_dir: &Path) {
        let source_id = 1;
        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let source_code = fs::read_to_string(source_path.as_ref()).unwrap();

        let tokens = syntax::tokenize::tokenize(source_id, source_path, Rc::new(source_code));
        let ast_root = ast::parse::parse(tokens);

        write_snapshot(name, "ast.txt", tests_dir, |out| {
            write!(out, "{:#?}\n", ast_root).unwrap();
        });

        let kir_root = kir::gen::gen(ast_root);
        // snapshot(name, "kir.txt", tests_dir, |out| {
        //     write!(out, "{:#?}\n", kir_root).unwrap();
        // });
    }

    #[test]
    fn snapshot_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        snapshot_test("assign", &tests_dir);
        snapshot_test("exit_42", &tests_dir);
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
}
