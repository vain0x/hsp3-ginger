//! 動作確認用のコード。HSPランタイムと接続することなく VSCode のデバッガーを動かす。

extern crate libc;
extern crate ws;

#[macro_use]
extern crate log;
#[cfg(test)]
extern crate env_logger;

#[cfg(target_os = "windows")]
extern crate winapi;

mod connection;
mod helpers;
mod hsprt;
mod hspsdk;
mod logger;

#[derive(Clone, Copy, Debug)]
struct HspDebugImpl {
    mode: hspsdk::DebugMode,
}

impl hsprt::HspDebug for HspDebugImpl {
    fn set_mode(&mut self, mode: hspsdk::DebugMode) {
        self.mode = mode;
    }
}

fn initialize() {
    // INFO 以上の重要度を持つメッセージが標準出力されるようにする。
    std::env::set_var("RUST_LOG", "info");

    // ロガー (info! など) を有効化する。
    env_logger::init();

    logger::init_mod();
    connection::init_mod();
}

fn main() {
    initialize();
    info!("initialized");

    let d = HspDebugImpl {
        mode: hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode,
    };

    connection::Connection::spawn(d);

    // FIXME: スレッドに join する。
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("quit");
}
