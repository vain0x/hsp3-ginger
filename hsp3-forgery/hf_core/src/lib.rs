pub(crate) mod ast;
pub(crate) mod cir;
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
        let source_path = tests_dir.join(format!("{}/{}.hsp", name, name));
        let source_code = fs::read_to_string(&source_path).unwrap();

        let tokens = syntax::tokenize::tokenize(source_id, Rc::new(source_code));
        let ast_root = ast::parse::parse(tokens);

        write_snapshot(name, "ast.txt", tests_dir, |out| {
            write!(out, "{:#?}\n", ast_root).unwrap();
        });

        let kir_root = kir::gen::gen(ast_root);
        // snapshot(name, "kir.txt", tests_dir, |out| {
        //     write!(out, "{:#?}\n", kir_root).unwrap();
        // });

        let c_module = cir::gen::gen(kir_root);
        write_snapshot(name, "cir.txt", tests_dir, |out| {
            write!(out, "{:#?}\n", c_module).unwrap();
        })
    }

    #[test]
    fn snapshot_tests() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        snapshot_test("assign", &tests_dir);
        snapshot_test("exit_42", &tests_dir);
    }
}
