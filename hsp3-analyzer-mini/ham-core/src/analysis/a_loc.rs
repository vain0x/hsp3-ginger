use super::{ADoc, ARange};

/// Location. テキストドキュメント上の範囲
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ALoc {
    pub(crate) doc: ADoc,
    pub(crate) range: ARange,
}
