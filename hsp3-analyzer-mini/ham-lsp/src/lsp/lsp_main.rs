use crate::lsp::*;
use std::io;
use std::path::PathBuf;

fn init_log() {
    use log::LevelFilter;
    use simplelog::*;
    use std::fs::OpenOptions;

    let log_filter = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let file_path = if cfg!(debug_assertions) {
        PathBuf::from("ham-lsp.log")
    } else {
        std::env::temp_dir().join("ham-lsp.log")
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

    let stdin = io::stdin();
    let stdin = stdin.lock();
    let receiver = LspReceiver::new(stdin);
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let sender = LspSender::new(stdout);
    let handler = LspHandler::new(sender, hsp_root);
    handler.main(receiver);
}
