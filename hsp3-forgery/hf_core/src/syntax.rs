pub(crate) mod keyword;
pub(crate) mod location;
pub(crate) mod pun;
pub(crate) mod source_loader;
pub(crate) mod syntax_source;
pub(crate) mod token;
pub(crate) mod tokenize;
pub(crate) mod tokenize_context;
pub(crate) mod tokenize_rules;

pub(crate) use crate::framework::*;
pub(crate) use crate::source::*;
pub(crate) use location::Location;
pub(crate) use source_loader::*;
pub(crate) use syntax_source::*;
pub(crate) use token::{Token, TokenData};
pub(crate) use tokenize::TokensComponent;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn test_tokenize() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let source_file_id = Id::new(1);
        let source_path = Rc::new(tests_dir.join("syntax/syntax.hsp"));

        let mut source_files = SourceFileComponent::new();
        source_files.insert(
            source_file_id,
            SourceFile {
                source_path: source_path.clone(),
            },
        );

        let source_code = fs::read_to_string(source_path.as_ref()).unwrap();
        let source = SyntaxSource {
            source_file_id,
            source_files: &source_files as *const SourceFileComponent,
        };

        let tokens = tokenize::tokenize(source, Rc::new(source_code));

        let mut snapshot = vec![];

        for token in tokens {
            write!(snapshot, "{:?} `{}`\n", token.token(), token.text()).unwrap();
        }

        fs::write(
            &tests_dir.join("syntax/syntax_snapshot_tokenize.txt"),
            snapshot,
        )
        .unwrap();
    }
}
