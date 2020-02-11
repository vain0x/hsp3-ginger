use crate::file_changes::FileChanges;
use crate::file_watcher::FileWatcher;
use lsp_types::*;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct LspModel {
    editing_files: HashSet<PathBuf>,
    file_watcher: Option<FileWatcher>,
    file_changes: FileChanges,
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
        if self.file_changes.disconnected {
            self.file_watcher.take();
        }

        self.file_changes.clear();
    }

    pub(crate) fn open_doc(&mut self, uri: Url, _version: u64, _text: String) {
        let file_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => file_path,
        };

        self.editing_files.insert(file_path);
        self.poll();
    }

    pub(crate) fn change_doc(&mut self, uri: Url, _version: u64, _text: String) {
        let file_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => file_path,
        };

        self.editing_files.insert(file_path);
        self.poll();
    }

    pub(crate) fn close_doc(&mut self, uri: Url) {
        let file_path = match canonicalize_uri(uri) {
            None => return,
            Some(file_path) => file_path,
        };

        self.editing_files.remove(&file_path);
        self.poll();
    }

    pub(crate) fn completion(&mut self, _uri: Url, _position: Position) -> CompletionList {
        CompletionList {
            is_incomplete: true,
            items: vec![],
        }
    }

    pub(crate) fn definitions(&mut self, _uri: Url, _position: Position) -> Vec<Location> {
        vec![]
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

    pub(crate) fn validate(&mut self, _uri: Url) -> Vec<Diagnostic> {
        vec![]
    }
}

fn canonicalize_uri(uri: Url) -> Option<PathBuf> {
    uri.to_file_path()
        .ok()
        .and_then(|path| path.canonicalize().ok())
}
