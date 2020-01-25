pub(crate) mod keyword;
pub(crate) mod location;
pub(crate) mod position;
pub(crate) mod pun;
pub(crate) mod range;
pub(crate) mod text_cursor;
pub(crate) mod token;
pub(crate) mod tokenize;
pub(crate) mod tokenize_context;
pub(crate) mod tokenize_rules;

pub(crate) use location::SourceLocation;
pub(crate) use position::Position;
pub(crate) use range::Range;
pub(crate) use text_cursor::TextCursor;
pub(crate) use token::{Token, TokenData};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn test_tokenize() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let source_id = 1;
        let source_code = fs::read_to_string(&tests_dir.join("syntax/syntax.hsp")).unwrap();

        let tokens = tokenize::tokenize(source_id, Rc::new(source_code));

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

    #[test]
    fn test_parse() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let source_id = 1;
        let source_code = fs::read_to_string(&tests_dir.join("exit_42/exit_42.hsp")).unwrap();

        let tokens = tokenize::tokenize(source_id, Rc::new(source_code));
        let ast_root = crate::ast::parse::parse(tokens);

        let mut snapshot = vec![];
        write!(snapshot, "{:#?}\n", ast_root).unwrap();

        fs::write(
            &tests_dir.join("exit_42/exit_42_snapshot_ast.txt"),
            snapshot,
        )
        .unwrap();
    }
}
