//! ライブラリの外部に公開する関数など。
//! API 仕様はすべて未確定。おそらく後で変わる。

use crate::analysis::*;
use crate::parse;
use crate::source::*;
use crate::syntax::*;
use crate::token::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub enum DiagnosticSeverity {
    Error,
}

#[derive(Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub range: TextRange,
    pub message: String,
}

#[derive(Clone)]
pub struct TextPosition {
    pub line: usize,
    pub character: usize,
}

impl From<Position> for TextPosition {
    fn from(it: Position) -> Self {
        Self {
            line: it.line,
            character: it.character,
        }
    }
}

impl From<TextPosition> for Position {
    fn from(it: TextPosition) -> Self {
        Self {
            line: it.line,
            character: it.character,
        }
    }
}

#[derive(Clone)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl From<Range> for TextRange {
    fn from(it: Range) -> Self {
        Self {
            start: it.start.into(),
            end: it.end.into(),
        }
    }
}

impl From<TextRange> for Range {
    fn from(it: TextRange) -> Self {
        Self {
            start: it.start.into(),
            end: it.end.into(),
        }
    }
}

#[derive(Clone)]
pub struct DocHandle(SourceFile);
#[derive(Clone)]
pub struct SyntaxRootHandle(Rc<SyntaxRoot>);

#[derive(Default)]
pub struct World {
    empty_string: Rc<String>,
    source_files: HashSet<SourceFile>,
    source_codes: HashMap<SourceFile, Rc<SourceCode>>,
    syntax_roots: HashMap<TokenSource, (Rc<SourceCode>, Rc<SyntaxRoot>)>,
}

impl World {
    pub fn new() -> Self {
        World::default()
    }

    pub fn add_source_file(&mut self, source_path: Rc<PathBuf>) {
        let source_file = SourceFile { source_path };

        self.source_files.insert(source_file);
    }

    pub fn remove_source_file(&mut self, source_path: Rc<PathBuf>) {
        let source_file = SourceFile { source_path };
        let token_source = TokenSource::from_file(source_file.clone());

        self.source_files.remove(&source_file);

        // 関連データも削除する。
        self.source_codes.remove(&source_file);
        self.syntax_roots.remove(&token_source);
    }

    fn get_source_code(&mut self, source_file: &SourceFile) -> Rc<SourceCode> {
        self.source_codes
            .get(&source_file)
            .cloned()
            .unwrap_or_else(|| self.empty_string.clone())
    }

    pub fn set_source_code(&mut self, source_path: Rc<PathBuf>, source_code: SourceCode) {
        let source_file = SourceFile { source_path };
        let token_source = TokenSource::from_file(source_file.clone());

        if self.get_source_code(&source_file).as_str() == source_code.as_str() {
            return;
        }

        // 依存データを削除する。
        self.syntax_roots.remove(&token_source);

        self.source_codes.insert(source_file, Rc::new(source_code));
    }

    fn require_syntax_root(&mut self, source_file: SourceFile) -> Rc<SyntaxRoot> {
        let source_code = self.get_source_code(&source_file);
        let token_source = TokenSource::from_file(source_file);

        // ソースコードに変更がなければ構文木を使いまわす。
        if let Some(syntax_root) =
            self.syntax_roots
                .get(&token_source)
                .and_then(|(old_one, syntax_root)| {
                    if Rc::ptr_eq(&old_one, &source_code) {
                        Some(syntax_root)
                    } else {
                        None
                    }
                })
        {
            return syntax_root.clone();
        }

        let tokens = tokenize::tokenize(token_source.clone(), source_code.clone());
        let syntax_root = parse::parse_tokens(&tokens);
        self.syntax_roots
            .insert(token_source, (source_code, syntax_root.clone()));
        syntax_root
    }

    pub fn get_diagnostics(&mut self, source_path: Rc<PathBuf>, diagnostics: &mut Vec<Diagnostic>) {
        let source_file = SourceFile { source_path };
        let syntax_root = self.require_syntax_root(source_file);

        let mut add_error = |range: Range, message: String| {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                range: range.into(),
                message,
            });
        };

        for element in syntax_root.node().descendant_elements() {
            match element {
                SyntaxElement::Token(token) => {
                    if token.kind() == Token::Other {
                        add_error(token.location().range, "この文字を解釈できません。(注意: フォージェリはまだ全角文字を解釈できません。)".to_string());
                    }
                }
                SyntaxElement::Node(node) => match node.kind() {
                    NodeKind::LabelLiteral => {
                        if !node.child_tokens().any(|token| {
                            token.kind() == Token::Ident || token.kind() == Token::IdentAtSign
                        }) {
                            add_error(node.range(), "ラベル名がありません。".to_string());
                        }
                    }
                    NodeKind::CallExpr => {
                        if let Some(left_paren) = node
                            .child_tokens()
                            .filter(|token| token.kind() == Token::LeftParen)
                            .next()
                        {
                            if node
                                .child_tokens()
                                .all(|token| token.kind() != Token::RightParen)
                            {
                                add_error(
                                    left_paren.range(),
                                    "カッコが閉じていません。".to_string(),
                                );
                            }
                        }
                    }
                    NodeKind::Param => {
                        if !node
                            .child_tokens()
                            .any(|token| token.kind() == Token::Ident)
                        {
                            add_error(node.range(), "パラメータタイプがありません。".to_string());
                        }
                    }
                    NodeKind::Other => {
                        add_error(node.range(), "この部分は文法的に解釈できないので、無視しています。(注意: フォージェリはまだ HSP3 の一部の機能にしか対応していません。)"
                                .to_string());
                    }
                    _ => continue,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    fn write_snapshot(name: &str, suffix: &str, tests_dir: &Path, f: impl Fn(&mut Vec<u8>)) {
        let mut out = vec![];
        f(&mut out);

        let file_path = tests_dir.join(format!("{}/{}_snapshot_{}", name, name, suffix));
        fs::write(&file_path, out).unwrap();
    }

    #[test]
    fn test_diagnostics() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tests_dir = root_dir.join("../tests");
        let test_names = vec!["assign", "command", "exit_42", "syntax_error", "syntax_pp"];

        for name in test_names {
            let source_path = Rc::new(tests_dir.join(format!("{}/{}.hsp", name, name)));

            let mut world = World::new();
            world.add_source_file(source_path.clone());

            let source_code = fs::read_to_string(source_path.as_ref()).unwrap();
            world.set_source_code(source_path.clone(), source_code);

            let mut diagnostics = vec![];
            world.get_diagnostics(source_path, &mut diagnostics);

            write_snapshot(name, "diagnostics.txt", &tests_dir, |out| {
                for diagnostic in &diagnostics {
                    let range = Range::from(diagnostic.range.clone());
                    write!(out, "{}.hsp:{} {:?}\n", name, range, diagnostic.message).unwrap();
                }
            });
        }
    }
}
