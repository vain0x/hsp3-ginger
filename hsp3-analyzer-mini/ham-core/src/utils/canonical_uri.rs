use normalize_path::NormalizePath;
use std::{
    fmt::{self, Debug},
    path::{Path, PathBuf},
};

/// 正規化済みのURI
///
/// **正規化** について:
/// `a/../b` と `b` のように、等価だが異なる表現を統一的な表現に揃える。
/// (シンボリックリンクの展開なども行う。)
///
/// (理由):
/// 正規化されていない URL をマップのキーに使ってしまうと、
/// 単一のファイルに対して複数のデータを登録できてしまい、不具合の原因になる。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalUri {
    uri: lsp_types::Url,
}

impl CanonicalUri {
    /// 絶対パス → URI に変換する
    pub(crate) fn from_abs_path(abs_path: &Path) -> Option<Self> {
        assert!(abs_path.is_absolute(), "absolute path only");

        // canonicalize に成功するなら、これを正規形と信じて使う。
        if let Ok(c_path) = abs_path.canonicalize() {
            if let Ok(uri) = lsp_types::Url::from_file_path(c_path) {
                return Some(CanonicalUri { uri });
            }
        }

        // ファイルが存在しない場合などに canonicalize は失敗する。
        // 中間の `..` などの余分なパスだけ除去してURIとして使う。
        let n_path = NormalizePath::normalize(abs_path);
        if let Ok(uri) = lsp_types::Url::from_file_path(n_path) {
            return Some(CanonicalUri { uri });
        }

        None
    }

    pub(crate) fn from_url(url: &lsp_types::Url) -> Self {
        // `file` スキームで絶対パス指定の場合は、ファイルパスの正規化処理を行う。
        if url.scheme() == "file" {
            if let Ok(path) = url.to_file_path() {
                if path.is_absolute() {
                    if let Some(uri) = Self::from_abs_path(&path) {
                        return uri;
                    }
                }
            }
        }

        // 正規形か分からないが、そのまま使う。
        CanonicalUri { uri: url.clone() }
    }

    pub(crate) fn into_url(self) -> lsp_types::Url {
        self.uri
    }

    pub(crate) fn to_file_path(&self) -> Option<PathBuf> {
        if self.uri.scheme() == "file" {
            self.uri.to_file_path().ok()
        } else {
            None
        }
    }
}

impl Debug for CanonicalUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uri)
    }
}
