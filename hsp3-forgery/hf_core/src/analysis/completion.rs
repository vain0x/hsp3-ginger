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
    let ast_root = crate::ast::parse::parse(&tokens);

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

pub(crate) struct SignatureHelp {
    pub(crate) params: Vec<String>,
    pub(crate) active_param_index: usize,
}

pub(crate) fn signature_help(
    source_id: SourceId,
    position: Position,
    project: &mut Project,
) -> Option<SignatureHelp> {
    let source = project.sources.get(source_id)?;

    let tokens = crate::syntax::tokenize::tokenize(
        source_id,
        Rc::clone(&source.source_path),
        Rc::clone(&source.source_code),
    );
    let ast_root = crate::ast::parse::parse(&tokens);

    use crate::ast::*;

    fn on_expr(
        a: &AExpr,
        p: Position,
        out: &mut Option<SignatureHelp>,
        accept: &impl Fn(&mut Option<SignatureHelp>),
    ) -> bool {
        match a {
            AExpr::Int(expr) => {
                // FIXME: トークンに接触していなくても引数領域の範囲内ならシグネチャヘルプは反応するべき
                if expr.token.location.range.contains_loosely(p) {
                    accept(out);
                    return true;
                }

                false
            }
        }
    }

    fn on_arg(
        a: &AArg,
        p: Position,
        out: &mut Option<SignatureHelp>,
        accept: &impl Fn(&mut Option<SignatureHelp>),
    ) -> bool {
        if let Some(expr) = &a.expr_opt {
            if on_expr(expr, p, out, accept) {
                return true;
            }
        }

        if let Some(comma) = &a.comma_opt {
            if p == comma.location.start() {
                accept(out);
                return true;
            }
        }

        false
    }

    fn on_stmt(a: &AStmtNode, p: Position, out: &mut Option<SignatureHelp>) -> bool {
        // FIXME: assign/return も関数の引数のシグネチャヘルプを表示できる可能性があるので内部に入るべき
        match a {
            AStmtNode::Assign(stmt) => false,
            AStmtNode::Command(stmt) => {
                for (i, arg) in stmt.args.iter().enumerate() {
                    if on_arg(arg, p, out, &|out| {
                        *out = Some(SignatureHelp {
                            params: vec!["x".to_string(), "y".to_string()],
                            active_param_index: i,
                        });
                    }) {
                        return true;
                    }
                }
                false
            }
            AStmtNode::Return(..) => false,
        }
    }

    fn go_node(a: &ANodeData, p: Position, out: &mut Option<SignatureHelp>) -> bool {
        match &a.node {
            ANode::Stmt(node) => on_stmt(node, p, out),
            ANode::Fn { .. } | ANode::Module { .. } | ANode::Root { .. } => {
                if go_nodes(&a.children, p, out) {
                    return true;
                }
                false
            }
        }
    }

    fn go_nodes(nodes: &[ANodeData], p: Position, out: &mut Option<SignatureHelp>) -> bool {
        for node in nodes {
            if go_node(node, p, out) {
                return true;
            }
        }

        false
    }

    let mut signature_help = None;
    go_node(&ast_root, position, &mut signature_help);
    signature_help
}
