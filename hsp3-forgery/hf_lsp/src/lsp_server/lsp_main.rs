use super::*;
use crate::lsp_model::LspModel;
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
        PathBuf::from("hsp3-forgery-lsp.log")
    } else {
        std::env::temp_dir().join("hsp3-forgery-lsp.log")
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

pub(crate) fn start_lsp_server(hsp_root: PathBuf) -> ! {
    init_log();

    let stdin = io::stdin();
    let stdin = stdin.lock();
    let receiver = LspReceiver::new(stdin);
    let stdout = io::stdout();
    let stdout = stdout.lock();
    let sender = LspSender::new(stdout);
    let model = LspModel::new(hsp_root);
    let handler = LspHandler::new(sender, model);
    handler.main(receiver)
}
