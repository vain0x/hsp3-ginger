use crate::{
    parse::{parse_root, PRoot, PToken, PVisitor},
    source::DocId,
    utils::{rc_str::RcStr, read_file::read_file},
};
use rowan::Language;
use std::{
    io::{stdin, stdout, Read, Write as _},
    path::PathBuf,
};

/// `ham parse`: ファイルを構文解析して構文木を出力する (出力内容は仮)
pub fn parse_subcommand(files: Vec<String>) {
    assert!(!files.is_empty());

    for (file_index, filename) in files.into_iter().enumerate() {
        let text = if filename == "-" {
            let mut buf = String::new();
            stdin().read_to_string(&mut buf).unwrap();
            buf
        } else {
            let mut buf = String::new();
            if !read_file(&PathBuf::from(&filename), &mut buf) {
                panic!("ERROR: Cannot read {filename:?}");
            }
            buf
        };

        let doc: DocId = (file_index as usize) + 1;
        let text = RcStr::from(text);

        let tokens = crate::token::tokenize(doc, text);
        let tokens = PToken::from_tokens(tokens.into());
        let root = parse_root(tokens);

        let green = to_rowan::green_node(&root);
        let node: rowan::SyntaxNode<HspLang> = rowan::SyntaxNode::new_root(green);

        let mut out = stdout().lock();
        write!(out, "file: {filename}\n{node:#?}").unwrap();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum HspLang {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MySyntaxKind {
    Token,
    Stmt,
}

impl rowan::api::Language for HspLang {
    type Kind = MySyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            1 => MySyntaxKind::Token,
            2 => MySyntaxKind::Stmt,
            _ => unreachable!(),
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        let n = match kind {
            MySyntaxKind::Token => 1,
            MySyntaxKind::Stmt => 2,
        };
        rowan::SyntaxKind(n)
    }
}

struct V<'a> {
    builder: rowan::GreenNodeBuilder<'a>,
}

impl<'a> PVisitor for V<'a> {
    fn on_token(&mut self, token: &PToken) {
        let kind = HspLang::kind_to_raw(MySyntaxKind::Token);
        for t in token
            .leading
            .iter()
            .chain(std::iter::once(token.body.as_ref()))
            .chain(token.trailing.iter())
        {
            self.builder.token(kind, &t.text);
        }
    }

    fn on_stmt(&mut self, stmt: &crate::parse::PStmt) {
        let kind = HspLang::kind_to_raw(MySyntaxKind::Stmt);
        self.builder.start_node(kind);
        self.on_stmt_default(stmt);
        self.builder.finish_node()
    }
}

mod to_rowan {
    use super::*;

    pub(super) fn green_node(root: &PRoot) -> rowan::GreenNode {
        let mut v = V {
            builder: rowan::GreenNodeBuilder::new(),
        };
        let kind = HspLang::kind_to_raw(MySyntaxKind::Stmt); // root
        v.builder.start_node(kind);
        v.on_root(root);
        v.builder.finish_node();
        v.builder.finish()
    }
}
