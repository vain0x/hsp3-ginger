use super::{LspHandler, LspReceiver, LspSender};
use crate::lang_service::{LangService, LangServiceOptions};
use std::{
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

fn get_options_from_env() -> LangServiceOptions {
    LangServiceOptions {
        lint_enabled: std::env::var("HAM_LINT").map_or(true, |s| !s.is_empty()),
        watcher_enabled: true,
    }
}

pub fn start_lsp_server(hsp3_home: PathBuf) {
    init_log();

    let stdin = stdin();
    let stdin = stdin.lock();
    let receiver = LspReceiver::new(stdin);
    let stdout = stdout();
    let stdout = stdout.lock();
    let sender = LspSender::new(stdout);
    let lang_service = LangService::new(hsp3_home, get_options_from_env());
    let handler = LspHandler::new(sender, lang_service);
    handler.main(receiver);
}
