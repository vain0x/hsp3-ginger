use crate::framework::*;
use crate::syntax::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Workspace {
    workspace_id: Id<Workspace>,
}

pub(crate) type SourceComponent = HashMap<Workspace, Vec<SyntaxSource>>;

impl Workspace {
    /// 1つのファイルだけからなるワークスペースを生成し、
    /// そのワークスペースと、その唯一のソースファイルを返す。
    pub(crate) fn new_with_file(
        source_path: Rc<PathBuf>,
        sources: &mut SourceComponent,
        ids: &mut IdProvider,
    ) -> (Workspace, SyntaxSource) {
        let workspace_id = ids.fresh();
        let workspace = Workspace { workspace_id };

        let source_id = ids.fresh();
        let source = SyntaxSource {
            source: Source {
                source_id,
                source_path,
            },
        };

        sources
            .entry(workspace.clone())
            .or_default()
            .push(source.clone());

        (workspace, source)
    }
}
