pub(crate) mod keyword;
pub(crate) mod location;
pub(crate) mod pun;
pub(crate) mod syntax_source;
pub(crate) mod token;
pub(crate) mod tokenize;
pub(crate) mod tokenize_context;
pub(crate) mod tokenize_rules;

pub(crate) use crate::framework::*;
pub(crate) use crate::source::*;
pub(crate) use location::Location;
pub(crate) use syntax_source::*;
pub(crate) use token::{Token, TokenData};
pub(crate) use tokenize::TokensComponent;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;
    use crate::world::{self, World};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn test_tokenize() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let mut w = World::new();
        let source_path = Rc::new(tests_dir.join("syntax/syntax.hsp"));

        let (_workspace, source_file_id) =
            Workspace::new_with_file(source_path, &mut w.source_files, &mut w.ids);

        world::load_source_codes(&mut w);
        world::tokenize(&mut w);

        let tokens = w
            .tokenss
            .get(&SyntaxSource::from_file(source_file_id, &w.source_files))
            .unwrap();

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
