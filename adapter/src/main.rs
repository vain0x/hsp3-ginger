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
mod hspsdk;
mod logger;

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

    connection::Connection::spawn();

    // FIXME: スレッドに join する。
    std::thread::sleep(std::time::Duration::from_secs(10));

    info!("quit");
}
