use lsp_types::Url;
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

// 正規化処理を行った後の uniform resource identifier (URI)
//
// 正規化について:
// `a/../b` と `b` のように、等価だが異なる表現を統一的な表現に揃える。
// (シンボリックリンクの展開なども行う。)
//
// ファイルパスの参照先が存在しない場合、
//
// 正規化されていない URL をマップのキーに使ってしまうと、
// 単一のファイルに対して複数のデータが登録できてしまい、不具合の原因になる。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalUri {
    uri: Url,
}

impl CanonicalUri {
    pub(crate) fn from_file_path(path: &Path) -> Option<Self> {
        let to_uri = |path: &Path| {
            Url::from_file_path(path)
                .ok()
                .map(|uri| CanonicalUri { uri })
        };

        // canonicalize に成功するなら、これを正規系と信じて使う。
        if let Ok(canonical_path) = path.canonicalize() {
            return to_uri(&canonical_path);
        }

        // 絶対パスでなければ、カレントディレクトリと繋ぐ。
        // 念のため再び canonicalize を試してから、絶対パスを URI にする。
        if !path.is_absolute() {
            if let Some(path) = current_dir().ok().map(|current_dir| current_dir.join(path)) {
                if let Ok(canonical_path) = path.canonicalize() {
                    return to_uri(&canonical_path);
                }

                return to_uri(&path);
            }
        }

        // canonicalize できない絶対パスというのはよく分からないが、
        // 例えばファイルパスを取得した直後にファイルが削除された場合などに発生しうる。
        // (存在しないファイルパスは canonicalize に失敗するはず。)
        // 正規系ではないかもしれないが、そのまま URI として使う。
        to_uri(path)
    }

    pub(crate) fn from_url(uri: &Url) -> Self {
        let uri = match uri
            .to_file_path()
            .ok()
            .and_then(|file_path| file_path.canonicalize().ok())
            .and_then(|canonical_path| Url::from_file_path(canonical_path).ok())
        {
            Some(uri) => uri,
            None => uri.to_owned(),
        };
        CanonicalUri { uri }
    }

    pub(crate) fn into_url(self) -> Url {
        self.uri
    }

    pub(crate) fn to_file_path(&self) -> Option<PathBuf> {
        self.uri.to_file_path().ok()
    }
}
