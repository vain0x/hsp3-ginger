use crate::framework::*;
use crate::token::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Workspace {
    workspace_id: Id<Workspace>,
}

impl Workspace {
    /// 1つのファイルだけからなるワークスペースを生成し、
    /// そのワークスペースと、その唯一のソースファイルを返す。
    pub(crate) fn new_with_file(
        source_path: Rc<PathBuf>,
        ids: &mut IdProvider,
    ) -> (Workspace, SourceFile) {
        let workspace_id = ids.fresh();
        let workspace = Workspace { workspace_id };

        let source_file = SourceFile { source_path };

        (workspace, source_file)
    }
}
