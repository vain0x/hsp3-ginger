use crate::file_changes::FileChanges;
use crate::file_watcher::FileWatcher;
use hsp3_forgery_core::api::World;
use lsp_types::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct LspModel {
    editing_files: HashSet<Rc<PathBuf>>,
    file_watcher: Option<FileWatcher>,
    file_changes: FileChanges,
    world: World,
    hsp_root: PathBuf,
}

impl LspModel {
    pub(crate) fn new(hsp_root: PathBuf) -> Self {
        LspModel {
            hsp_root,
            ..Default::default()
        }
    }

    pub(crate) fn did_initialize(&mut self) {
        self.file_watcher = FileWatcher::start();
        self.poll();
    }

    pub(crate) fn shutdown(&mut self) {
        self.file_watcher.take();
    }

    fn poll(&mut self) {
        if let Some(file_watcher) = self.file_watcher.as_mut() {
            file_watcher.poll();
            file_watcher.swap_changes(&mut self.file_changes);
            self.on_file_changes();
        }
    }

    fn on_file_changes(&mut self) {
        for updated_path in &self.file_changes.updated_paths {
            if self.editing_files.contains(updated_path)
                || self.file_changes.removed_paths.contains(updated_path)
            {
                continue;
            }

            if let Some(source_code) = read_file(updated_path) {
                let source_path = Rc::new(updated_path.to_owned());
                self.world.set_source_code(source_path, source_code);
            }
        }

        for removed_path in &self.file_changes.removed_paths {
            self.world.remove_source_file(Rc::new(removed_path.clone()));
        }

        if self.file_changes.disconnected {
            self.file_watcher.take();
        }

        self.file_changes.clear();
    }

    pub(crate) fn open_doc(&mut self, uri: Url, _version: u64, text: String) {
        let source_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => Rc::new(file_path),
        };

        self.editing_files.insert(source_path.clone());
        self.world.set_source_code(source_path, text);

        self.poll();
    }

    pub(crate) fn change_doc(&mut self, uri: Url, _version: u64, text: String) {
        let source_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => Rc::new(file_path),
        };

        self.editing_files.insert(source_path.clone());
        self.world.set_source_code(source_path, text);

        self.poll();
    }

    pub(crate) fn close_doc(&mut self, uri: Url) {
        let source_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => Rc::new(file_path),
        };

        self.editing_files.remove(&source_path);
        self.world.remove_source_file(source_path);

        self.poll();
    }

    pub(crate) fn completion(&mut self, _uri: Url, _position: Position) -> CompletionList {
        CompletionList {
            is_incomplete: true,
            items: vec![],
        }
    }

    pub(crate) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();

        let source_path = match canonicalize_uri(uri) {
            None => return vec![],
            Some(source_path) => Rc::new(source_path),
        };

        let location = match self
            .world
            .goto_definition(source_path, from_position(position))
            .and_then(to_location)
        {
            None => return vec![],
            Some(location) => location,
        };

        vec![location]
    }

    pub(crate) fn highlights(&mut self, _uri: Url, _position: Position) -> Vec<DocumentHighlight> {
        vec![]
    }

    pub(crate) fn hover(&mut self, _uri: Url, _position: Position) -> Option<Hover> {
        None
    }

    pub(crate) fn references(
        &mut self,
        _uri: Url,
        _position: Position,
        _include_definition: bool,
    ) -> Vec<Location> {
        vec![]
    }

    pub(crate) fn validate(&mut self, uri: Url) -> Vec<Diagnostic> {
        self.poll();

        let mut diagnostics = vec![];

        if let Some(source_path) = canonicalize_uri(uri) {
            let mut forgery_diagnostics = vec![];
            self.world
                .get_diagnostics(Rc::new(source_path), &mut forgery_diagnostics);

            diagnostics.extend(forgery_diagnostics.into_iter().map(|d| Diagnostic {
                severity: Some(DiagnosticSeverity::Error),
                message: d.message,
                range: to_range(d.range),
                ..Default::default()
            }))
        }

        diagnostics
    }
}

fn canonicalize_uri(uri: Url) -> Option<PathBuf> {
    uri.to_file_path()
        .ok()
        .and_then(|path| path.canonicalize().ok())
}

fn to_position(position: hsp3_forgery_core::api::TextPosition) -> Position {
    Position {
        line: position.line as u64,
        character: position.character as u64,
    }
}

fn from_position(position: Position) -> hsp3_forgery_core::api::TextPosition {
    hsp3_forgery_core::api::TextPosition {
        line: position.line as usize,
        character: position.character as usize,
    }
}

fn to_range(range: hsp3_forgery_core::api::TextRange) -> Range {
    Range {
        start: to_position(range.start),
        end: to_position(range.end),
    }
}

fn from_range(range: Range) -> hsp3_forgery_core::api::TextRange {
    hsp3_forgery_core::api::TextRange {
        start: from_position(range.start),
        end: from_position(range.end),
    }
}

fn to_location(location: hsp3_forgery_core::api::TextLocation) -> Option<Location> {
    let uri = Url::from_file_path(location.source_path.as_ref()).ok()?;
    Some(Location::new(uri, to_range(location.range)))
}

fn read_file(file_path: &Path) -> Option<String> {
    debug!("read_file {:?}", file_path);
    let data = std::fs::read(file_path).ok()?;

    let mut out = String::new();
    if let Err(err) = crate::text_encoding::decode_as_shift_jis_or_utf8(&data, &mut out) {
        warn!("ファイルの読込 {:?}", err);
        return None;
    }

    Some(out)
}
