use crate::help_source::collect_all_symbols;
use crate::sem::{self, ProjectSem};
use crate::syntax::{self, DocId};
use encoding::{
    codec::utf_8::UTF8Encoding, label::encoding_from_windows_code_page, DecoderTrap, Encoding,
    StringWriter,
};
use lsp_types::*;
use notify::{DebouncedEvent, RecommendedWatcher};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError};

/// テキストドキュメントのバージョン番号 (エディタ上で編集されるたびに変わる番号。いつの状態のテキストドキュメントを指しているかを明確にするためのもの。)
type TextDocumentVersion = i64;

#[derive(Default)]
pub(super) struct LspModel {
    last_doc: usize,
    doc_to_uri: HashMap<DocId, Url>,
    uri_to_doc: HashMap<Url, DocId>,
    open_docs: HashSet<DocId>,
    doc_versions: HashMap<DocId, TextDocumentVersion>,
    sem: ProjectSem,
    hsp_root: PathBuf,
    file_watcher: Option<RecommendedWatcher>,
    file_event_rx: Option<Receiver<DebouncedEvent>>,
}

fn canonicalize_uri(uri: Url) -> Url {
    uri.to_file_path()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .and_then(|path| Url::from_file_path(path).ok())
        .unwrap_or(uri)
}

fn file_ext_is_watched(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "hsp" || ext == "as")
}

/// ファイルを shift_jis または UTF-8 として読む。
fn read_file(file_path: &Path, out: &mut impl StringWriter, shift_jis: &dyn Encoding) -> bool {
    let content = match fs::read(file_path).ok() {
        None => return false,
        Some(x) => x,
    };

    shift_jis
        .decode_to(&content, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&content, DecoderTrap::Strict, out))
        .is_ok()
}

fn loc_to_range(loc: syntax::Loc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start.row as u64, loc.start.col as u64),
        Position::new(loc.end.row as u64, loc.end.col as u64),
    )
}

fn plain_text_to_marked_string(text: String) -> MarkedString {
    const PLAIN_LANG_ID: &str = "plaintext";

    MarkedString::LanguageString(LanguageString {
        language: PLAIN_LANG_ID.to_string(),
        value: text,
    })
}

impl LspModel {
    pub(super) fn new(hsp_root: PathBuf) -> Self {
        Self {
            hsp_root,
            sem: sem::ProjectSem::new(),
            ..Default::default()
        }
    }

    fn fresh_doc(&mut self) -> DocId {
        self.last_doc += 1;
        DocId::new(self.last_doc)
    }

    fn to_loc(&self, uri: &Url, position: Position) -> Option<syntax::Loc> {
        let doc = self.uri_to_doc.get(uri).cloned()?;

        // FIXME: position は UTF-16 ベース、pos は UTF-8 ベースなので、マルチバイト文字が含まれている場合は変換が必要
        let pos = syntax::Pos {
            row: position.line as usize,
            col: position.character as usize,
        };

        Some(syntax::Loc {
            doc,
            start: pos,
            end: pos,
        })
    }

    fn loc_to_location(&self, loc: syntax::Loc) -> Option<Location> {
        let uri = self.doc_to_uri.get(&loc.doc)?.clone();
        let range = loc_to_range(loc);
        Some(Location { uri, range })
    }

    pub(super) fn did_initialize(&mut self) {
        debug!("hsphelp ファイルからシンボルを探索します。");
        let mut file_count = 0;
        let mut symbols = vec![];
        let mut warnings = vec![];
        collect_all_symbols(&self.hsp_root, &mut file_count, &mut symbols, &mut warnings)
            .map_err(|e| warn!("{}", e))
            .ok();
        for w in warnings {
            warn!("{}", w);
        }

        let doc = self.fresh_doc();

        let symbols = symbols
            .into_iter()
            .enumerate()
            .map(|(i, symbol)| {
                Rc::new(sem::Symbol {
                    symbol_id: self.sem.last_symbol_id + i + 1,
                    name: symbol.name.into(),
                    kind: sem::SymbolKind::Command {
                        local: false,
                        ctype: false,
                    },
                    details: sem::SymbolDetails {
                        description: symbol.description.map(|s| s.into()),
                        documentation: symbol.documentation.clone(),
                    },
                    scope: sem::Scope::new_global(doc),
                })
            })
            .collect::<Vec<_>>();

        self.sem.last_symbol_id += symbols.len();

        self.sem.add_hs_symbols(doc, symbols);

        self.scan_files();

        if let Some((file_watcher, file_event_rx)) = self.start_file_watcher() {
            self.file_watcher = Some(file_watcher);
            self.file_event_rx = Some(file_event_rx);
        }
    }

    fn scan_files(&mut self) -> Option<()> {
        let current_dir = std::env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let glob_pattern = format!("{}/**/*.hsp", current_dir.to_str()?);

        debug!("ファイルリストを取得します '{}'", glob_pattern);

        let entries = match glob::glob(&glob_pattern) {
            Err(err) => {
                warn!("ファイルリストの取得 {:?}", err);
                return None;
            }
            Ok(entries) => entries,
        };

        for entry in entries {
            match entry {
                Err(err) => warn!("ファイルエントリの取得 {:?}", err),
                Ok(path) => {
                    self.change_file(&path);
                }
            }
        }

        None
    }

    fn start_file_watcher(&mut self) -> Option<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
        debug!("ファイルウォッチャーを起動します");

        use notify::{RecursiveMode, Watcher};
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let delay_millis = 1000;

        let current_dir = std::env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let (tx, rx) = channel();

        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(delay_millis))
            .map_err(|err| warn!("ファイルウォッチャーの作成 {:?}", err))
            .ok()?;

        watcher
            .watch(&current_dir, RecursiveMode::Recursive)
            .map_err(|err| warn!("ファイルウォッチャーの起動 {:?}", err))
            .ok()?;

        debug!("ファイルウォッチャーを起動しました ({:?})", current_dir);
        Some((watcher, rx))
    }

    fn poll(&mut self) {
        let rx = match self.file_event_rx.as_mut() {
            None => return,
            Some(rx) => rx,
        };

        debug!("ファイルウォッチャーのイベントをポールします。");

        let mut rescan = false;
        let mut updated_paths = HashSet::new();
        let mut removed_paths = HashSet::new();
        let mut disconnected = false;

        loop {
            match rx.try_recv() {
                Ok(DebouncedEvent::Create(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが作成されました: {:?}", path);
                    updated_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Write(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが変更されました: {:?}", path);
                    updated_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Remove(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが削除されました: {:?}", path);
                    removed_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Rename(ref src_path, ref dest_path)) => {
                    debug!("ファイルが移動しました: {:?} → {:?}", src_path, dest_path);
                    if file_ext_is_watched(src_path) {
                        removed_paths.insert(src_path.clone());
                    }
                    if file_ext_is_watched(dest_path) {
                        updated_paths.insert(dest_path.clone());
                    }
                }
                Ok(DebouncedEvent::Rescan) => {
                    debug!("ファイルウォッチャーから再スキャンが要求されました");
                    rescan = true;
                }
                Ok(ev) => {
                    debug!("ファイルウォッチャーのイベントをスキップします: {:?}", ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if rescan {
            self.scan_files();
        } else {
            for path in updated_paths {
                if removed_paths.contains(&path) {
                    continue;
                }
                self.change_file(&path);
            }

            for path in removed_paths {
                self.close_file(&path);
            }
        }

        if disconnected {
            self.shutdown_file_watcher();
        }
    }

    fn shutdown_file_watcher(&mut self) {
        debug!("ファイルウォッチャーがシャットダウンしました。");
        self.file_watcher.take();
        self.file_event_rx.take();
    }

    pub(super) fn shutdown(&mut self) {
        self.shutdown_file_watcher();
    }

    fn do_change_doc(&mut self, uri: Url, version: i64, text: String) {
        debug!("追加または変更されたファイルを解析します {:?}", uri);

        let doc = match self.uri_to_doc.get(&uri) {
            None => {
                let doc = self.fresh_doc();
                self.doc_to_uri.insert(doc, uri.clone());
                self.uri_to_doc.insert(uri, doc);
                doc
            }
            Some(&doc) => doc,
        };

        self.doc_versions.insert(doc, version);
        self.sem.update_doc(doc, text.into());
    }

    fn do_close_doc(&mut self, uri: Url) {
        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.sem.close_doc(doc);
            self.doc_to_uri.remove(&doc);
        }

        self.uri_to_doc.remove(&uri);
    }

    pub(super) fn open_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = canonicalize_uri(uri);

        self.do_change_doc(uri.clone(), version, text);

        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.open_docs.insert(doc);
        }

        self.poll();
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = canonicalize_uri(uri);

        self.do_change_doc(uri.clone(), version, text);

        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.open_docs.insert(doc);
        }

        self.poll();
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = canonicalize_uri(uri);

        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.open_docs.remove(&doc);
        }

        self.poll();
    }

    pub(super) fn change_file(&mut self, path: &Path) -> Option<()> {
        let shift_jis = encoding_from_windows_code_page(932).or_else(|| {
            warn!("shift_jis エンコーディングの取得");
            None
        })?;

        let uri = Url::from_file_path(path)
            .map_err(|err| warn!("URL の作成 {:?} {:?}", path, err))
            .ok()?;
        let uri = canonicalize_uri(uri);

        let is_open = self
            .uri_to_doc
            .get(&uri)
            .map_or(false, |doc| self.open_docs.contains(&doc));
        if is_open {
            debug!("ファイルは開かれているのでロードされません。");
            return None;
        }

        let mut text = String::new();
        if !read_file(path, &mut text, shift_jis) {
            warn!("ファイルを開けません {:?}", path);
        }

        let version = 1;
        self.do_change_doc(uri, version, text);

        None
    }

    pub(super) fn close_file(&mut self, path: &Path) -> Option<()> {
        let uri = Url::from_file_path(path)
            .map_err(|err| warn!("URL の作成 {:?} {:?}", path, err))
            .ok()?;

        let uri = canonicalize_uri(uri);

        self.do_close_doc(uri);

        None
    }

    fn do_completion(&mut self, uri: &Url, position: Position) -> Option<CompletionList> {
        let mut items = vec![];
        let mut symbols = vec![];

        let loc = self.to_loc(uri, position)?;

        self.sem.get_symbol_list(loc.doc, loc.start, &mut symbols);

        for symbol in symbols {
            let kind = match symbol.kind {
                sem::SymbolKind::Macro { ctype: true, .. }
                | sem::SymbolKind::Command { ctype: true, .. } => CompletionItemKind::Function,
                sem::SymbolKind::Label | sem::SymbolKind::Macro { .. } => {
                    CompletionItemKind::Constant
                }
                sem::SymbolKind::Command { .. } => CompletionItemKind::Method, // :thinking_face:
                sem::SymbolKind::Param { .. } | sem::SymbolKind::Static => {
                    CompletionItemKind::Variable
                }
            };

            items.push(CompletionItem {
                kind: Some(kind),
                label: symbol.name.to_string(),
                detail: symbol.details.description.as_ref().map(|s| s.to_string()),
                documentation: if symbol.details.documentation.is_empty() {
                    None
                } else {
                    Some(Documentation::String(
                        symbol.details.documentation.join("\r\n\r\n"),
                    ))
                },
                filter_text: if symbol.name.as_str().starts_with("#") {
                    Some(symbol.name.as_str().chars().skip(1).collect::<String>())
                } else {
                    None
                },
                data: Some(serde_json::to_value(&symbol.symbol_id).unwrap()),
                ..CompletionItem::default()
            })
        }

        Some(CompletionList {
            is_incomplete: false,
            items,
        })
    }

    pub(super) fn completion(&mut self, uri: Url, position: Position) -> CompletionList {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_completion(&uri, position)
            .unwrap_or(CompletionList {
                is_incomplete: true,
                items: vec![],
            })
    }

    fn do_definitions(&mut self, uri: Url, position: Position) -> Option<Vec<Location>> {
        let loc = self.to_loc(&uri, position)?;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];

        self.sem.get_symbol_defs(symbol_id, &mut locs);

        Some(
            locs.into_iter()
                .filter_map(|loc| self.loc_to_location(loc))
                .collect(),
        )
    }

    pub(super) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_definitions(uri, position).unwrap_or(vec![])
    }

    fn do_highlights(&mut self, uri: Url, position: Position) -> Option<Vec<DocumentHighlight>> {
        let loc = self.to_loc(&uri, position)?;
        let doc = loc.doc;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];
        let mut highlights = vec![];

        self.sem.get_symbol_defs(symbol_id, &mut locs);
        highlights.extend(
            locs.drain(..)
                .map(|loc| (DocumentHighlightKind::Write, loc)),
        );

        self.sem.get_symbol_uses(symbol_id, &mut locs);
        highlights.extend(locs.drain(..).map(|loc| (DocumentHighlightKind::Read, loc)));

        highlights.retain(|(_, loc)| loc.doc == doc);

        Some(
            highlights
                .into_iter()
                .map(|(kind, loc)| DocumentHighlight {
                    kind: Some(kind),
                    range: loc_to_range(loc),
                })
                .collect(),
        )
    }

    pub(super) fn highlights(&mut self, uri: Url, position: Position) -> Vec<DocumentHighlight> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_highlights(uri, position).unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();
        let uri = canonicalize_uri(uri);

        let loc = self.to_loc(&uri, position)?;
        let (symbol, symbol_loc) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut contents = vec![];
        contents.push(plain_text_to_marked_string(symbol.name.to_string()));

        if let Some(description) = symbol.details.description.as_ref() {
            contents.push(plain_text_to_marked_string(description.to_string()));
        }

        contents.extend(
            symbol
                .details
                .documentation
                .iter()
                .map(|text| plain_text_to_marked_string(text.to_string())),
        );

        {
            let mut locs = vec![];
            self.sem.get_symbol_defs(symbol_id, &mut locs);
            let def_links = locs
                .iter()
                .filter_map(|&loc| {
                    let location = self.loc_to_location(loc)?;
                    let uri = location
                        .uri
                        .to_string()
                        .replace("%3A", ":")
                        .replace("\\", "/");
                    let Position { line, character } = location.range.start;
                    Some(format!("- [{}:{}:{}]({})", uri, line, character, uri))
                })
                .collect::<Vec<_>>();
            if !def_links.is_empty() {
                contents.push(MarkedString::from_markdown(def_links.join("\r\n")));
            }
        }

        Some(Hover {
            contents: HoverContents::Array(contents),
            range: Some(loc_to_range(symbol_loc)),
        })
    }

    fn do_references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Option<Vec<Location>> {
        let loc = self.to_loc(&uri, position)?;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];

        if include_definition {
            self.sem.get_symbol_defs(symbol_id, &mut locs);
        }
        self.sem.get_symbol_uses(symbol_id, &mut locs);

        Some(
            locs.into_iter()
                .filter_map(|loc| self.loc_to_location(loc))
                .collect(),
        )
    }

    pub(super) fn references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_references(uri, position, include_definition)
            .unwrap_or(vec![])
    }

    pub(super) fn rename(
        &mut self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<HashMap<Url, Vec<TextEdit>>> {
        self.poll();
        let uri = canonicalize_uri(uri);

        // common ディレクトリのファイルは変更しない。
        if uri.as_str().contains("common") {
            return None;
        }

        // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
        let locs = {
            let loc = self.to_loc(&uri, position)?;
            let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
            let symbol_id = symbol.symbol_id;

            let mut locs = vec![];
            self.sem.get_symbol_defs(symbol_id, &mut locs);
            self.sem.get_symbol_uses(symbol_id, &mut locs);
            if locs.is_empty() {
                return None;
            }
            locs
        };

        // 名前変更の編集手順を構築する。(シンボルが書かれている位置をすべて新しい名前で置き換える。)
        let changes = {
            let mut changes = HashMap::new();
            for loc in locs {
                let location = match self.loc_to_location(loc) {
                    Some(location) => location,
                    None => continue,
                };

                let (uri, range) = (location.uri, location.range);
                let text_edit = TextEdit {
                    range,
                    new_text: new_name.to_string(),
                };

                changes.entry(uri).or_insert(vec![]).push(text_edit);
            }
            changes
        };

        Some(changes)
    }

    pub(super) fn validate(&mut self, _uri: Url) -> Vec<Diagnostic> {
        // self.poll();
        // let uri = canonicalize_uri(uri);

        // features::diagnostics::sem_to_diagnostics(&analysis.sem)
        vec![]
    }
}
