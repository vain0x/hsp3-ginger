use super::*;

pub(crate) enum EntryPoints {
    NonCommon,
    Docs(Vec<DocId>),
}

impl Default for EntryPoints {
    fn default() -> Self {
        EntryPoints::NonCommon
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectAnalysisRef<'a> {
    pub(super) def_sites: &'a [(SymbolRc, Loc)],
    pub(super) use_sites: &'a [(SymbolRc, Loc)],
}

impl<'a> ProjectAnalysisRef<'a> {
    pub(crate) fn locate_symbol(self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        self.def_sites
            .iter()
            .chain(self.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn collect_symbol_defs(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in self.def_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in self.use_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct IncludeGraph {
    edges: HashMap<DocId, Vec<DocId>>,
    // 辺をすべて逆向きにしたもの
    rev: HashMap<DocId, Vec<DocId>>,
}

impl IncludeGraph {
    #[allow(unused)]
    pub(crate) fn generate(wa: &AnalysisRef<'_>, docs: &lang_service::docs::Docs) -> Self {
        let mut it = Self::default();
        generate_include_graph(wa, docs, &mut it);
        it
    }
}

/// ファイル間の `include` の関係を表す有向グラフを構築する
#[allow(unused)]
fn generate_include_graph(
    wa: &AnalysisRef<'_>,
    docs: &lang_service::docs::Docs,
    include_graph: &mut IncludeGraph,
) {
    let get_name = |doc: DocId| match docs
        .get_uri(doc)
        .and_then(|uri| uri.to_file_path())
        .and_then(|path| {
            path.components()
                .last()
                .map(|x| x.as_os_str().to_string_lossy().to_string())
        }) {
        Some(it) => format!("{}:{}", doc, it),
        None => format!("{}", doc),
    };

    for (&src_doc, da) in wa.doc_analysis_map {
        let src_name = get_name(src_doc);
        // eprintln!("  >{}:{} ({})", src_doc, src_name, da.includes.len());

        for (included_name, l) in &da.includes {
            let included_doc = match wa.project_docs.find(included_name, Some(src_doc)) {
                Some(it) => it,
                None => {
                    debug!(
                        "include unresolved: {}:{} ({})",
                        src_name, l.range, included_name
                    );
                    continue;
                }
            };
            debug!("include {}:{} -> {}", src_name, l.range, included_name);

            include_graph
                .edges
                .entry(src_doc)
                .or_default()
                .push(included_doc);
        }
    }

    // dedup
    for (_, included_docs) in include_graph.edges.iter_mut() {
        included_docs.sort();
        included_docs.dedup();
    }

    // 逆向き
    for (&src_doc, target_docs) in include_graph.edges.iter() {
        for &target_doc in target_docs {
            include_graph
                .rev
                .entry(target_doc)
                .or_default()
                .push(src_doc);
        }
    }

    // dedup
    for (_, src_docs) in include_graph.rev.iter_mut() {
        src_docs.sort();
        src_docs.dedup();
    }
}

#[derive(Clone)]
pub(crate) struct CollectSymbolQuery {
    pub(crate) def_site: bool,
    pub(crate) use_site: bool,
}

impl Default for CollectSymbolQuery {
    fn default() -> Self {
        Self {
            def_site: true,
            use_site: true,
        }
    }
}

/// シンボルの定義箇所・出現箇所を列挙する
/// (到達可能性を用いて出力を削減する)
///
/// ドキュメント間に有向パスがある場合に到達可能であるとする
/// (例えば x → y → z のとき x と z は互いに到達可能である。
///  x → y, w → y のとき x と w は到達可能ではない)
///
/// 基準となるドキュメント(`doc`)と到達可能関係で結ばれているドキュメント内の定義箇所・使用箇所は列挙される。
/// (命令などの定義は後方参照が可能なので、基準となるドキュメントから定義箇所へのパスだけでなく、その逆のパスも使う。
///  使用側も後方参照が可能なので同様に扱う)
#[allow(unused)]
pub(crate) fn collect_symbols2(
    wa: &AnalysisRef<'_>,
    docs: &lang_service::docs::Docs,
    include_graph: &IncludeGraph,
    doc: DocId,
    query: CollectSymbolQuery,
    symbols: &mut Vec<SymbolRc>,
) {
    let mut reachable: HashSet<DocId> = HashSet::new();

    let mut stack = vec![];
    let mut done = HashSet::new();

    // forward
    {
        stack.push(doc);
        while let Some(doc) = stack.pop() {
            if !done.insert(doc) {
                continue;
            }

            reachable.insert(doc);

            if let Some(target_docs) = include_graph.edges.get(&doc) {
                stack.extend(target_docs);
            }
        }
    }

    // backward
    {
        stack.clear();
        done.clear();

        stack.push(doc);
        while let Some(doc) = stack.pop() {
            if !done.insert(doc) {
                continue;
            }

            reachable.insert(doc);

            if let Some(target_docs) = include_graph.rev.get(&doc) {
                stack.extend(target_docs);
            }
        }
    }

    let is_reachable = |doc: DocId| reachable.contains(&doc);

    if query.def_site {
        for (symbol, loc) in wa.def_sites {
            if is_reachable(loc.doc) {
                symbols.push(symbol.clone());
            }
        }
    }
    if query.use_site {
        for (symbol, loc) in wa.use_sites {
            if is_reachable(loc.doc) {
                symbols.push(symbol.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang_service::{docs::NO_VERSION, DocDb, LangService};
    use lsp_types::Url;

    fn dummy_url(s: &str) -> Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws").join(s);
        Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    fn add_doc(ls: &mut LangService, name: &str, text: &str) -> (DocId, CanonicalUri) {
        let url = dummy_url(name);
        let uri = CanonicalUri::from_url(&url);
        ls.open_doc(url, NO_VERSION, text.to_string());
        let doc = ls.find_doc_by_uri(&uri).unwrap();
        (doc, uri)
    }

    fn format_symbols(symbols: &[SymbolRc]) -> String {
        let mut names = symbols.iter().map(|s| s.name()).collect::<Vec<_>>();
        names.sort();
        names.join(", ")
    }

    #[test]
    fn test_reachable() {
        let mut ls = LangService::new_standalone();

        // 次のような依存関係がある:
        //      main -> mod_x
        //      mod_x_tests -> mod_x
        //      isolation
        //
        // mod_x, mod_x_tests, main の3つのドキュメントは、
        // どこでも mod_x で定義される命令 f を参照できる
        // mox_x_tests のなかでは、`test_main` がみえる
        // main と mod_x_tests の間にパスはないので、
        // main を基準とするとき `test_main` はみえない

        let mx = add_doc(
            &mut ls,
            "mod_x.hsp",
            r#"
#module
#deffunc f int a, str b
    return
#global
"#,
        );

        let mx_tests = add_doc(
            &mut ls,
            "mod_x_tests.hsp",
            r#"
#include "mod_x.hsp"

#module
#deffunc test_main
    f 0, 0
    return
#global

    test_main
"#,
        );

        let main = add_doc(
            &mut ls,
            "main.hsp",
            r#"
#include "mod_x.hsp"

#module
#deffunc app_main
    f 1, 1
    return
#global

    app_main
"#,
        );

        let isolation = add_doc(
            &mut ls,
            "isolation.hsp",
            r#"
#module
#deffunc isolated_f
#global

    isolated_f
"#,
        );

        let (wa, docs) = ls.analyze_for_test();
        let wa = &wa;
        let mut include_graph = IncludeGraph::default();
        generate_include_graph(wa, docs, &mut include_graph);

        let def_only = CollectSymbolQuery {
            def_site: true,
            use_site: false,
            ..Default::default()
        };

        // mod_x基準で定義箇所を列挙する:
        // 下流であるmain, mod_x_testsの両方がみえる
        let mut symbols = vec![];
        collect_symbols2(
            wa,
            docs,
            &include_graph,
            mx.0,
            def_only.clone(),
            &mut symbols,
        );
        assert_eq!(format_symbols(&symbols), "a, app_main, b, f, test_main");
        symbols.clear();

        // main基準で定義箇所を列挙する: mod_xにある命令fがみえる
        let mut symbols = vec![];
        collect_symbols2(
            wa,
            docs,
            &include_graph,
            main.0,
            def_only.clone(),
            &mut symbols,
        );
        assert_eq!(format_symbols(&symbols), "a, app_main, b, f");
        symbols.clear();

        // mod_x_tests基準で定義箇所を列挙する
        let mut symbols = vec![];
        collect_symbols2(
            wa,
            docs,
            &include_graph,
            mx_tests.0,
            def_only.clone(),
            &mut symbols,
        );
        assert_eq!(format_symbols(&symbols), "a, b, f, test_main");
        symbols.clear();

        // isolation基準で定義箇所を列挙する。
        // 到達可能関係がないため、ほかのドキュメントのシンボルはみえない
        let mut symbols = vec![];
        collect_symbols2(
            wa,
            docs,
            &include_graph,
            isolation.0,
            def_only.clone(),
            &mut symbols,
        );
        assert_eq!(format_symbols(&symbols), "isolated_f");
        symbols.clear();
    }
}
