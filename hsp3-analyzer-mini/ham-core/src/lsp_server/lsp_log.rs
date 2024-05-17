use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::{env::temp_dir, fs::OpenOptions, path::PathBuf};

pub(crate) fn init_log() {
    let log_filter = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let file_path = if cfg!(debug_assertions) {
        PathBuf::from("ham-lsp.log")
    } else {
        temp_dir().join("ham-lsp.log")
    };

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .expect("log file creation");

    WriteLogger::init(log_filter, Config::default(), file).expect("init log");
}
