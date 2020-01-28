use crate::syntax::*;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct IdProvider {
    last_id: AtomicUsize,
}

impl IdProvider {
    pub(crate) fn new() -> Self {
        IdProvider::default()
    }

    pub(crate) fn fresh(&self) -> usize {
        self.last_id.fetch_add(1, Ordering::Relaxed)
    }
}

pub(crate) type SourceId = usize;

pub(crate) struct Source {
    pub(crate) source_id: SourceId,
    pub(crate) source_path: Rc<PathBuf>,
    pub(crate) source_code: Rc<String>,
}

#[derive(Default)]
pub(crate) struct SourceArena {
    sources: HashMap<SourceId, Source>,
    path_to_ids: HashMap<PathBuf, SourceId>,
}

impl SourceArena {
    pub(crate) fn get(&self, source_id: SourceId) -> Option<&Source> {
        self.sources.get(&source_id)
    }

    pub(crate) fn path_to_id(&self, source_path: &Path) -> Option<SourceId> {
        self.path_to_ids.get(source_path).cloned()
    }
}

#[derive(Default)]
pub(crate) struct Project {
    pub(crate) sources: SourceArena,
}

impl Project {
    pub(crate) fn new() -> Self {
        Project::default()
    }
}

pub(crate) fn load_source(
    source_path: Rc<PathBuf>,
    id_provider: &IdProvider,
    sources: &mut SourceArena,
) -> io::Result<()> {
    let source_code = fs::read_to_string(source_path.as_ref())?;
    let source_id = id_provider.fresh();
    let source = Source {
        source_id,
        source_path: Rc::clone(&source_path),
        source_code: Rc::new(source_code),
    };

    sources.sources.insert(source_id, source);
    sources
        .path_to_ids
        .insert(PathBuf::clone(&*source_path), source_id);

    Ok(())
}

pub(crate) fn get_completion_list(
    source_id: SourceId,
    position: Position,
    project: &mut Project,
) -> Vec<String> {
    let source = match project.sources.get(source_id) {
        None => return vec![],
        Some(source) => source,
    };

    let tokens = crate::syntax::tokenize::tokenize(
        source_id,
        Rc::clone(&source.source_path),
        Rc::clone(&source.source_code),
    );
    let ast_root = crate::ast::parse::parse(tokens);

    use crate::ast::*;

    fn on_stmt(a: &AStmtNode, idents: &mut Vec<String>) {
        match a {
            AStmtNode::Assign(stmt) => {
                idents.push(stmt.left.text().to_string());
            }
            AStmtNode::Command(stmt) => {
                // FIXME: 実装
            }
            AStmtNode::Return(..) => {}
        }
    }

    fn go_node(a: &ANodeData, idents: &mut Vec<String>) {
        match &a.node {
            ANode::Stmt(node) => on_stmt(node, idents),
            ANode::Fn { .. } | ANode::Module { .. } | ANode::Root { .. } => {
                go_nodes(&a.children, idents)
            }
        }
    }

    fn go_nodes(nodes: &[ANodeData], idents: &mut Vec<String>) {
        for node in nodes {
            go_node(node, idents);
        }
    }

    let mut symbols = vec![];
    go_node(&ast_root, &mut symbols);
    symbols.sort();
    symbols.dedup();

    symbols
}
