use super::*;
use crate::{parse::*, source::range_is_touched};

macro_rules! or {
    ($opt:expr, $alt:expr) => {
        match $opt {
            Some(it) => it,
            None => $alt,
        }
    };
}

pub(crate) enum Diagnostic {
    Undefined,
    VarRequired,
}

struct Ctx {
    use_site_map: HashMap<(DocId, Pos), ASymbol>,
    diagnostics: Vec<(Diagnostic, Loc)>,
}

impl Ctx {
    fn symbol(&self, loc: Loc) -> Option<ASymbol> {
        self.use_site_map.get(&(loc.doc, loc.start())).cloned()
    }
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(_) => {}
        PStmt::Assign(_) => {}
        PStmt::Command(stmt) => {
            let loc = stmt.command.body.loc;
            let symbol = match ctx.symbol(loc) {
                Some(it) => it,
                None => {
                    ctx.diagnostics.push((Diagnostic::Undefined, loc));
                    return;
                }
            };

            if let Some(signature_data) = symbol.signature_opt() {
                for (arg, (param, _, _)) in stmt.args.iter().zip(&signature_data.params) {
                    match param {
                        Some(PParamTy::Var) | Some(PParamTy::Modvar) | Some(PParamTy::Array) => {}
                        _ => continue,
                    }

                    let mut rval = false;
                    let mut expr_opt = arg.expr_opt.as_ref();
                    while let Some(expr) = expr_opt {
                        match expr {
                            PExpr::Compound(compound) => {
                                let name = &compound.name().body;

                                let symbol = match ctx.symbol(name.loc) {
                                    Some(it) => it,
                                    _ => break,
                                };

                                rval = match symbol.kind {
                                    ASymbolKind::Label
                                    | ASymbolKind::Const
                                    | ASymbolKind::Enum
                                    | ASymbolKind::DefFunc
                                    | ASymbolKind::DefCFunc
                                    | ASymbolKind::ModFunc
                                    | ASymbolKind::ModCFunc
                                    | ASymbolKind::ComInterface
                                    | ASymbolKind::ComFunc => true,
                                    ASymbolKind::Param(Some(param)) => match param {
                                        PParamTy::Var
                                        | PParamTy::Array
                                        | PParamTy::Modvar
                                        | PParamTy::Local => false,
                                        _ => true,
                                    },
                                    _ => false,
                                };
                                break;
                            }
                            PExpr::Paren(expr) => expr_opt = expr.body_opt.as_deref(),
                            _ => {
                                rval = true;
                                break;
                            }
                        }
                    }
                    if rval {
                        let range = match arg.expr_opt.as_ref() {
                            Some(expr) => expr.compute_range(),
                            None => stmt.command.body.loc.range,
                        };
                        let loc = loc.with_range(range);
                        ctx.diagnostics.push((Diagnostic::VarRequired, loc));
                    }
                }
            }
        }
        PStmt::DefFunc(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        PStmt::Module(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        _ => {}
    }
}

type DocAnalysisMap = HashMap<DocId, DocAnalysis>;

/// ワークスペースの外側のデータ
#[derive(Default)]
pub(crate) struct HostData {
    pub(crate) builtin_env: Rc<SymbolEnv>,
    pub(crate) common_docs: Rc<HashMap<String, DocId>>,
    pub(crate) entrypoints: Vec<DocId>,
}

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    dirty_docs: HashSet<DocId>,
    doc_texts: HashMap<DocId, RcStr>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    doc_analysis_map: DocAnalysisMap,
    project1: ProjectAnalysis,
    project_opt: Option<ProjectAnalysis>,
}

impl AWorkspaceAnalysis {
    pub(crate) fn initialize(&mut self, host_data: HostData) {
        let HostData {
            common_docs,
            builtin_env,
            entrypoints,
        } = host_data;

        self.project_opt = if !entrypoints.is_empty() {
            let mut p = ProjectAnalysis::default();
            p.entrypoints = EntryPoints::Docs(entrypoints);
            p.common_docs = common_docs.clone();
            p.public_env.builtin = builtin_env.clone();
            Some(p)
        } else {
            None
        };

        self.project1.entrypoints = EntryPoints::NonCommon;
        self.project1.common_docs = common_docs;
        self.project1.public_env.builtin = builtin_env;
    }

    pub(crate) fn update_doc(&mut self, doc: DocId, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, text);
        self.doc_analysis_map
            .entry(doc)
            .and_modify(|a| a.invalidate());
    }

    pub(crate) fn close_doc(&mut self, doc: DocId) {
        self.dirty_docs.insert(doc);
        self.doc_texts.remove(&doc);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn set_project_docs(&mut self, project_docs: Rc<HashMap<String, DocId>>) {
        for p in [Some(&mut self.project1), self.project_opt.as_mut()]
            .iter_mut()
            .flatten()
        {
            p.project_docs = project_docs.clone();
        }
    }

    fn compute(&mut self) {
        // eprintln!("compute (dirty={:?})", &self.dirty_docs);
        if self.dirty_docs.is_empty() {
            return;
        }

        self.project1.invalidate();
        if let Some(p) = self.project_opt.as_mut() {
            p.invalidate();
        }

        let mut doc_analysis_map = take(&mut self.doc_analysis_map);

        for doc in self.dirty_docs.drain() {
            let text = match self.doc_texts.get(&doc) {
                Some(text) => text,
                None => continue,
            };

            let tokens = crate::token::tokenize(doc, text.clone());
            let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
            let root = crate::parse::parse_root(p_tokens.to_owned());
            let preproc = crate::analysis::preproc::analyze_preproc(doc, &root);

            let da = doc_analysis_map.entry(doc).or_default();
            da.set_syntax(p_tokens, root);
            da.set_preproc(preproc);
        }

        self.doc_analysis_map = doc_analysis_map;

        // 以前の解析結果を捨てる:
        for p in [Some(&mut self.project1), self.project_opt.as_mut()]
            .iter_mut()
            .flatten()
        {
            p.compute(&self.doc_analysis_map);
        }

        assert_eq!(self.project1.diagnostics.len(), 0);
    }

    pub(crate) fn in_preproc(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_preproc(pos, tokens))
    }

    pub(crate) fn in_str_or_comment(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_str_or_comment(pos, tokens))
    }

    pub(crate) fn get_tokens(&mut self, doc: DocId) -> Option<(RcStr, RcSlice<PToken>, &PRoot)> {
        self.compute();

        let text = self.doc_texts.get(&doc)?;
        let da = self.doc_analysis_map.get(&doc)?;
        Some((text.clone(), da.tokens.clone(), da.tree_opt.as_ref()?))
    }

    pub(crate) fn get_ident_at(&mut self, doc: DocId, pos: Pos16) -> Option<(RcStr, Loc)> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        let token = match tokens.binary_search_by_key(&pos, |t| t.body.loc.start().into()) {
            Ok(i) => tokens[i].body.as_ref(),
            Err(i) => tokens
                .iter()
                .skip(i.saturating_sub(1))
                .take(3)
                .find_map(|t| {
                    if t.body.kind == TokenKind::Ident && range_is_touched(&t.body.loc.range, pos) {
                        Some(t.body.as_ref())
                    } else {
                        None
                    }
                })?,
        };
        Some((token.text.clone(), token.loc))
    }

    pub(crate) fn require_some_project(&mut self) -> ProjectAnalysisRef {
        self.compute();

        let p = self.project_opt.as_mut().unwrap_or(&mut self.project1);
        p.compute(&self.doc_analysis_map)
    }

    pub(crate) fn require_project_for_doc(&mut self, doc: DocId) -> ProjectAnalysisRef {
        self.compute();

        if let Some(p) = self.project_opt.as_mut() {
            if p.active_docs.contains(&doc) {
                debug_assert!(p.is_computed());
                return p.compute(&self.doc_analysis_map);
            }
        }

        debug_assert!(self.project1.is_computed());
        self.project1.compute(&self.doc_analysis_map)
    }

    pub(crate) fn diagnose(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        self.diagnose_precisely(diagnostics);
    }

    pub(crate) fn diagnose_syntax_lints(&mut self, lints: &mut Vec<(SyntaxLint, Loc)>) {
        self.compute();

        let p = match self.project_opt.as_ref() {
            Some(it) => it,
            None => return,
        };

        for (&doc, da) in self.doc_analysis_map.iter() {
            if !p.active_docs.contains(&doc) {
                continue;
            }

            // if !da.syntax_lint_done {
            //     debug_assert_eq!(da.syntax_lints.len(), 0);
            //     let tree = or!(da.tree_opt.as_ref(), continue);
            //     crate::analysis::syntax_linter::syntax_lint(&tree, &mut da.syntax_lints);
            //     da.syntax_lint_done = true;
            // }
            // lints.extend(da.syntax_lints.iter().cloned());

            let tree = or!(da.tree_opt.as_ref(), continue);
            crate::analysis::syntax_linter::syntax_lint(&tree, lints);
        }
    }

    pub(crate) fn diagnose_precisely(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        let p = match &self.project_opt {
            Some(it) => it,
            None => return,
        };

        // diagnose:

        let use_site_map = p
            .use_sites
            .iter()
            .map(|(symbol, loc)| ((loc.doc, loc.start()), symbol.clone()))
            .collect::<HashMap<_, _>>();

        let mut ctx = Ctx {
            use_site_map,
            diagnostics: vec![],
        };

        for (&doc, da) in self.doc_analysis_map.iter() {
            if !p.active_docs.contains(&doc) {
                continue;
            }

            let root = or!(da.tree_opt.as_ref(), continue);

            for stmt in &root.stmts {
                on_stmt(stmt, &mut ctx);
            }
        }

        // どのプロジェクトに由来するか覚えておく必要がある
        diagnostics.extend(ctx.diagnostics.into_iter().map(|(d, loc)| {
            let msg = match d {
                Diagnostic::Undefined => "定義が見つかりません",
                Diagnostic::VarRequired => "変数か配列の要素が必要です。",
            }
            .to_string();
            (msg, loc)
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::AWorkspaceAnalysis;
    use super::*;
    use crate::source::{DocId, Pos};

    /// `<|x|>` のようなマーカーを含む文字列を受け取る。間に挟まれている x の部分をマーカーの名前と呼ぶ。
    /// マーカーを取り除いた文字列 text と、text の中でマーカーが指している位置のリストを返す。
    fn parse_cursor_string(s: &str) -> (String, Vec<(&str, Pos)>) {
        let mut output = vec![];

        let mut text = String::with_capacity(s.len());
        let mut pos = Pos::default();
        let mut i = 0;

        while let Some(offset) = s[i..].find("<|") {
            // カーソルを <| の手前まで進める。
            let j = i + offset;
            text += &s[i..j];
            pos += Pos::from(&s[i..j]);
            i += offset + "<|".len();

            // <| と |> の間を名前として取る。
            let name_len = s[i..].find("|>").expect("missing |>");
            let j = i + name_len;
            let name = &s[i..j];
            i += name_len + "|>".len();

            output.push((name, pos));
        }

        text += &s[i..];
        (text, output)
    }

    #[test]
    fn test_locate_static_var_def() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc: DocId = 1;
        let text = r#"
            <|A|>foo = 1
        "#;
        let expected_map = vec![("A", Some("foo"))]
            .into_iter()
            .collect::<HashMap<_, _>>();
        let (text, cursors) = parse_cursor_string(text);

        wa.update_doc(doc, text.into());

        let p = wa.require_project_for_doc(doc);

        for (name, pos) in cursors {
            let actual = p
                .locate_symbol(doc, pos.into())
                .map(|(symbol, _)| symbol.name());
            assert_eq!(actual.as_deref(), expected_map[name], "name={}", name);
        }
    }

    #[test]
    fn test_it_works() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc: DocId = 1;
        let text = r#"
            #module
            #deffunc <|A|>hello
                mes "Hello, world!"
                return
            #global

                <|B|>hello
                hello<|C|> <|D|>
        "#;
        let expected_map = vec![
            ("A", Some("hello")),
            ("B", Some("hello")),
            ("C", Some("hello")),
            ("D", None),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let (text, cursors) = parse_cursor_string(text);

        wa.update_doc(doc, text.into());

        let p = wa.require_project_for_doc(doc);

        for (name, pos) in cursors {
            let actual = p
                .locate_symbol(doc, pos.into())
                .map(|(symbol, _)| symbol.name());
            assert_eq!(actual.as_deref(), expected_map[name], "name={}", name);
        }
    }
}
