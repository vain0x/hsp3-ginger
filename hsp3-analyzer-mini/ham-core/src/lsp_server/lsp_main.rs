use super::{LspConfig, LspHandler, LspReceiver, LspSender};
use crate::lang_service::{LangService, LangServiceOptions};
use std::{
    env,
    io::{stdin, stdout},
    path::PathBuf,
};

pub(crate) fn init_log() {
    use log::LevelFilter;
    use simplelog::{Config, WriteLogger};
    use std::{env::temp_dir, fs::OpenOptions};

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

pub fn start_lsp_server(hsp3_root: PathBuf) {
    init_log();

    let lsp_config = LspConfig {
        watcher_enabled: env::var("HAM_WATCHER_ENABLED").map_or(true, |s| s == "1"),
    };

    let options = LangServiceOptions {
        lint_enabled: env::var("HAM_LINT").map_or(true, |s| s == "1"),
    };

    let stdin = stdin();
    let stdin = stdin.lock();
    let receiver = LspReceiver::new(stdin);
    let stdout = stdout();
    let stdout = stdout.lock();
    let sender = LspSender::new(stdout);
    let lang_service = LangService::new(hsp3_root, options);
    let handler = LspHandler::new(lsp_config, sender, lang_service);
    handler.main(receiver);
}
