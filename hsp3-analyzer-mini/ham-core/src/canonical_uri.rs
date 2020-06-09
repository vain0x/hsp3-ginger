use lsp_types::Url;
use std::path::{Path, PathBuf};

// 正規化された uniform resource identifier (URI)
// いまのところ実際にはファイルパスに等しい。
//
// 正規化について:
// 単一のローカルのファイルパスを表現する文字列は1つとは限らない。
// 例えば `./a.hsp` と `C:/a.hsp` は異なる文字列だが同じファイルを指しているかもしれない。
// ここでは同じファイルを指すパスが同じ文字列になるように変換している。
//
// 正規化されていないファイルパスをマップのキーに使ってしまうと、
// 単一のファイルに対して複数のデータが登録できてしまい、不具合の原因になる。
// 実際、ファイルウォッチャーから来るファイルパスと VSCode から渡されるファイルパスは、
// 同じファイルを指していても文字列としては異なるケースがみられた。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalUri {
    file_path: PathBuf,
}

impl CanonicalUri {
    pub(crate) fn from_file_path(path: &Path) -> Option<Self> {
        let path = path
            .canonicalize()
            .map_err(|err| warn!("CanonicalUri::from_file_path {:?}", err))
            .ok()?;
        Some(CanonicalUri { file_path: path })
    }

    pub(crate) fn from_url(url: &Url) -> Option<Self> {
        let path = url.to_file_path().ok()?;
        CanonicalUri::from_file_path(&path)
    }

    pub(crate) fn into_url(self) -> Option<Url> {
        Url::from_file_path(self.file_path).ok()
    }
}
