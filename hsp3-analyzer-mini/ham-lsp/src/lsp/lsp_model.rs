use crate::help_source::collect_all_symbols;
use crate::sem::{self, ProjectSem};
use crate::syntax::{self, DocId};
use lsp_types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Default)]
pub(super) struct LspModel {
    last_doc: usize,
    doc_to_uri: HashMap<DocId, Url>,
    uri_to_doc: HashMap<Url, DocId>,
    sem: ProjectSem,
    hsp_root: PathBuf,
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
                        description: symbol.description.map(|s| s.trim().to_string().into()),
                        documentation: symbol.documentation.clone(),
                    },
                    scope: sem::Scope::new_global(doc),
                })
            })
            .collect::<Vec<_>>();

        self.sem.last_symbol_id += symbols.len();

        self.sem.add_hs_symbols(doc, symbols);
    }

    pub(super) fn open_doc(&mut self, uri: Url, version: u64, text: String) {
        self.change_doc(uri, version, text.into());
    }

    pub(super) fn change_doc(&mut self, uri: Url, _version: u64, text: String) {
        let doc = match self.uri_to_doc.get(&uri) {
            None => {
                let doc = self.fresh_doc();
                self.doc_to_uri.insert(doc, uri.clone());
                self.uri_to_doc.insert(uri, doc);
                doc
            }
            Some(&doc) => doc,
        };

        self.sem.update_doc(doc, text.into());
    }

    pub(super) fn close_doc(&mut self, uri: &Url) {
        let doc = match self.uri_to_doc.get(uri) {
            None => return,
            Some(&doc) => doc,
        };

        self.sem.close_doc(doc);

        self.uri_to_doc.remove(uri);
        self.doc_to_uri.remove(&doc);
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

    pub(super) fn completion(&mut self, uri: &Url, position: Position) -> CompletionList {
        self.do_completion(uri, position).unwrap_or(CompletionList {
            is_incomplete: true,
            items: vec![],
        })
    }

    fn do_definitions(&mut self, uri: &Url, position: Position) -> Option<Vec<Location>> {
        let loc = self.to_loc(uri, position)?;
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

    pub(super) fn definitions(&mut self, uri: &Url, position: Position) -> Vec<Location> {
        self.do_definitions(uri, position).unwrap_or(vec![])
    }

    fn do_highlights(&mut self, uri: &Url, position: Position) -> Option<Vec<DocumentHighlight>> {
        let loc = self.to_loc(uri, position)?;
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

    pub(super) fn highlights(&mut self, uri: &Url, position: Position) -> Vec<DocumentHighlight> {
        self.do_highlights(uri, position).unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: &Url, position: Position) -> Option<Hover> {
        let loc = self.to_loc(uri, position)?;
        let (symbol, symbol_loc) = self.sem.locate_symbol(loc.doc, loc.start)?;

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

        Some(Hover {
            contents: HoverContents::Array(contents),
            range: Some(loc_to_range(symbol_loc)),
        })
    }

    fn do_references(
        &mut self,
        uri: &Url,
        position: Position,
        include_definition: bool,
    ) -> Option<Vec<Location>> {
        let loc = self.to_loc(uri, position)?;
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
        uri: &Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        self.do_references(uri, position, include_definition)
            .unwrap_or(vec![])
    }

    pub(super) fn validate(&mut self, _uri: &Url) -> Vec<Diagnostic> {
        // features::diagnostics::sem_to_diagnostics(&analysis.sem)
        vec![]
    }
}
