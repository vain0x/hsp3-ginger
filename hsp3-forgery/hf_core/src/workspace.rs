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
        source_files: &mut SourceFileComponent,
        sources: &mut SourceComponent,
        ids: &mut IdProvider,
    ) -> (Workspace, SyntaxSource) {
        let workspace_id = ids.fresh();
        let workspace = Workspace { workspace_id };

        let source_file_id = ids.fresh();
        source_files.insert(source_file_id, SourceFile { source_path });

        let source = SyntaxSource {
            source_file_id,
            source_files: source_files as *const SourceFileComponent,
        };

        sources
            .entry(workspace.clone())
            .or_default()
            .push(source.clone());

        (workspace, source)
    }
}
