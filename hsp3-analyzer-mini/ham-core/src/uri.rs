#![allow(unused)]

use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(crate) enum CanonicalUri {
    LocalFile(PathBuf),
}

impl CanonicalUri {
    pub(crate) fn from_path(path: &Path) -> io::Result<Self> {
        Ok(CanonicalUri::LocalFile(path.canonicalize()?))
    }

    pub(crate) fn as_path(&self) -> Option<&Path> {
        match self {
            CanonicalUri::LocalFile(path) => Some(path.as_path()),
        }
    }
}
