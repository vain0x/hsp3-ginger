use normalize_path::NormalizePath;
use std::path::{Path, PathBuf};

/// 正規化済みのURI
///
/// **正規化** について:
/// `a/../b` と `b` のように、等価だが異なる表現を統一的な表現に揃える。
/// (シンボリックリンクの展開なども行う。)
///
/// (理由):
/// 正規化されていない URL をマップのキーに使ってしまうと、
/// 単一のファイルに対して複数のデータを登録できてしまい、不具合の原因になる。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
        let uri = match url
            .to_file_path()
            .ok()
            .and_then(|file_path| file_path.canonicalize().ok())
            .and_then(|canonical_path| lsp_types::Url::from_file_path(canonical_path).ok())
        {
            Some(uri) => uri,
            None => url.to_owned(),
        };
        CanonicalUri { uri }
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
