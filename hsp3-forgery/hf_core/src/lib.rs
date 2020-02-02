pub(crate) mod analysis;
pub(crate) mod ast;
pub(crate) mod framework;
pub(crate) mod source;
pub(crate) mod syntax;
pub(crate) mod workspace;

pub(crate) use workspace::Workspace;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::*;
    use crate::syntax::*;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    fn load_sources(
        workspace: &Workspace,
        source_files: &SourceFileComponent,
        source_codes: &mut SourceCodeComponent,
    ) {
        syntax::source_loader::load_sources(&source_files, source_codes);
    }

    fn tokenize_sources(
        workspace: &Workspace,
        source_files: &SourceFileComponent,
        source_codes: &SourceCodeComponent,
        tokenss: &mut TokensComponent,
    ) {
        let mut ss = vec![];
        for (&source_file_id, source_code) in source_codes {
            let source = SyntaxSource::from_file(source_file_id, source_files);
            ss.push((source, source_code));
        }
        syntax::tokenize::tokenize_sources(&ss, tokenss);
    }

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
        let mut ids = IdProvider::new();
        let mut source_files = SourceFileComponent::default();

        let test_names = vec!["assign", "command", "exit_42", "syntax_error"];

        for name in test_names {
            let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

            let (workspace, source_file_id) =
                Workspace::new_with_file(source_path.clone(), &mut source_files, &mut ids);
            let mut source_codes = SourceCodeComponent::default();
            let mut tokenss = TokensComponent::default();

            load_sources(&workspace, &source_files, &mut source_codes);
            tokenize_sources(&workspace, &source_files, &source_codes, &mut tokenss);

            let source = SyntaxSource::from_file(source_file_id, &source_files);
            let tokens = tokenss.get(&source).unwrap();
            let ast_root = ast::parse::parse(tokens);

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

        let mut ids = IdProvider::new();
        let mut source_files = SourceFileComponent::default();
        let mut source_codes = SourceCodeComponent::default();
        let mut tokenss = TokensComponent::default();

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let (workspace, source_file_id) =
            Workspace::new_with_file(source_path, &mut source_files, &mut ids);

        load_sources(&workspace, &source_files, &mut source_codes);
        tokenize_sources(&workspace, &source_files, &source_codes, &mut tokenss);

        let source = SyntaxSource::from_file(source_file_id, &source_files);
        let tokens = tokenss.get(&source).unwrap();
        let ast_root = ast::parse::parse(tokens);

        let position = syntax::Position {
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

        let mut ids = IdProvider::new();
        let mut source_files = SourceFileComponent::default();
        let mut source_codes = SourceCodeComponent::default();
        let mut tokenss = TokensComponent::default();

        let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));
        let (workspace, source_file_id) =
            Workspace::new_with_file(source_path, &mut source_files, &mut ids);

        load_sources(&workspace, &source_files, &mut source_codes);
        tokenize_sources(&workspace, &source_files, &source_codes, &mut tokenss);

        let source = SyntaxSource::from_file(source_file_id, &source_files);
        let tokens = tokenss.get(&source).unwrap();
        let ast_root = ast::parse::parse(tokens);

        // first
        let position = syntax::Position {
            line: 0,
            character: 7,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(0));

        // second
        let position = syntax::Position {
            line: 0,
            character: 13,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert_eq!(signature_help_opt.map(|sh| sh.active_param_index), Some(1));

        // 範囲外
        let position = syntax::Position {
            line: 0,
            character: 1,
        };
        let signature_help_opt = crate::analysis::completion::signature_help(&ast_root, position);
        assert!(signature_help_opt.is_none());
    }
}
