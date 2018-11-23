//! 動作確認用のコード。HSPランタイムと接続することなく VSCode のデバッガーを動かす。

extern crate libc;
extern crate ws;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;
#[cfg(test)]
extern crate env_logger;

#[cfg(windows)]
extern crate winapi;

mod app;
mod connection;
mod debug_adapter_connection;
mod debug_adapter_protocol;
mod helpers;
mod hsprt;
mod hspsdk;
mod logger;

enum Event {
    Stop { line: i32 },
    Scope { frame_id: i32 },
}

#[derive(Clone, Copy, Debug)]
struct HspDebugImpl {
    mode: hspsdk::DebugMode,
}

impl hsprt::HspDebug for HspDebugImpl {
    fn terminate(&self) {}

    fn set_mode(&mut self, mode: hspsdk::DebugMode) {
        self.mode = mode;
    }

    fn get_globals(&self, seq: i64) {}
}

fn initialize() {
    // INFO 以上の重要度を持つメッセージが標準出力されるようにする。
    std::env::set_var("RUST_LOG", "info");

    // ロガー (info! など) を有効化する。
    env_logger::init();

    logger::init_mod();
}

fn main() {
    initialize();
    info!("initialized");

    let d = HspDebugImpl {
        mode: hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode,
    };

    // FIXME: スレッドに join する。
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("quit");
}
