use std::{ffi::OsStr, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lang {
    /// `.hsp` or `.as`
    Hsp3,

    /// `.hs`
    HelpSource,
}

impl Lang {
    fn from_extension(extension: &OsStr) -> Option<Lang> {
        match extension.to_str()? {
            "hsp" | "as" => Some(Lang::Hsp3),
            "hs" => Some(Lang::HelpSource),
            _ => None,
        }
    }

    pub(crate) fn from_path(path: &Path) -> Option<Lang> {
        let extension = path.extension()?;
        Lang::from_extension(extension)
    }
}
