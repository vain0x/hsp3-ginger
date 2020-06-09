use super::{LspHandler, LspReceiver, LspSender};
use crate::lang_service::LangService;
use std::{
    io::{stdin, stdout},
    path::PathBuf,
};

fn init_log() {
    use log::LevelFilter;
    use simplelog::{Config, WriteLogger};
    use std::{env::temp_dir, fs::OpenOptions};

    let log_filter = if cfg!(debug_assertions) {
        LevelFilter::Debug
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
        .append(true)
        .open(file_path)
        .expect("log file creation");

    WriteLogger::init(log_filter, Config::default(), file).expect("init log");

    info!("--------------------------------------------------");
    info!("                START NEW SESSION ");
    info!("--------------------------------------------------");
}

pub fn start_lsp_server(hsp_root: PathBuf) {
    init_log();

    let stdin = stdin();
    let stdin = stdin.lock();
    let receiver = LspReceiver::new(stdin);
    let stdout = stdout();
    let stdout = stdout.lock();
    let sender = LspSender::new(stdout);
    let lang_service = LangService::new(hsp_root);
    let handler = LspHandler::new(sender, lang_service);
    handler.main(receiver);
}
