pub(crate) mod fat_token;
pub(crate) mod keyword;
pub(crate) mod location;
pub(crate) mod pun;
pub(crate) mod token;
pub(crate) mod token_source;
pub(crate) mod tokenize;
pub(crate) mod tokenize_context;
pub(crate) mod tokenize_rules;
pub(crate) mod trivia;

pub(crate) use crate::source::*;
pub(crate) use fat_token::*;
pub(crate) use location::Location;
pub(crate) use token::{Token, TokenData};
pub(crate) use token_source::*;
pub(crate) use trivia::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn test_tokenize() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let mut source_files = HashSet::new();
        let mut source_codes = HashMap::new();
        let mut tokenss = HashMap::new();

        let source_path = Rc::new(tests_dir.join("syntax/syntax.hsp"));
        let source_file = SourceFile { source_path };
        source_files.insert(source_file.clone());

        world::load_source_codes(source_files.iter().cloned(), &mut source_codes);
        world::tokenize(&source_codes, &mut tokenss);

        let tokens = tokenss.get(&TokenSource::from_file(source_file)).unwrap();

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
