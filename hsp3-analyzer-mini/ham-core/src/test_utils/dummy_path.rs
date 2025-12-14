#![cfg(test)]
use std::path::PathBuf;

pub(crate) fn dummy_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from("Z:/no_exist")
    } else {
        PathBuf::from("/.no_exist")
    }
}
