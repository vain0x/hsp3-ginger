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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;
    use crate::world::{self, World};
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn test_tokenize() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");

        let mut w = World::new();
        let mut source_files = HashSet::new();
        let mut source_codes = HashMap::new();
        let mut tokenss = HashMap::new();

        let source_path = Rc::new(tests_dir.join("syntax/syntax.hsp"));
        let source_file = SourceFile { source_path };
        source_files.insert(source_file.clone());

        world::load_source_codes(source_files.iter().cloned(), &mut source_codes, &mut w);
        world::tokenize(&source_codes, &mut tokenss, &mut w);

        let tokens = tokenss.get(&SyntaxSource::from_file(source_file)).unwrap();

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
