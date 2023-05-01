//! VSCode 側のデバッガーアダプターと通信する。

use crate::app;
use log::{debug, error, info, warn};
use shared::{debug_adapter_connection as dac, debug_adapter_protocol as dap};
use std::{
    fs::{self, File},
    io,
    sync::mpsc,
    thread,
};

/// コネクションワーカーが扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Connect,
    Send(dap::Msg),
}

/// コネクションワーカーに処理を依頼するもの。
#[derive(Clone, Debug)]
pub(crate) struct Sender {
    sender: mpsc::Sender<Action>,
}

impl Sender {
    pub fn send(&self, action: Action) {
        self.sender
            .send(action)
            .map_err(|err| error!("[connection] {:?}", err))
            .ok();
    }
}

/// コネクションワーカー。
/// VSCode 側のデバッガーアダプターが立てている WebSocket サーバーに接続して双方向通信を行う。
pub(crate) struct Worker {
    app_sender: app::Sender,
    receiver: mpsc::Receiver<Action>,
    connection: Option<(File, thread::JoinHandle<()>)>,
}

impl Worker {
    pub fn new(app_sender: app::Sender) -> (Self, Sender) {
        let (sender, receiver) = mpsc::channel::<Action>();
        let sender = Sender { sender };

        let worker = Worker {
            app_sender,
            receiver,
            connection: None,
        };

        (worker, sender)
    }

    pub fn run(mut self) {
        loop {
            match self.receiver.recv() {
                Ok(Action::Connect) => {
                    // 接続要求が来たとき: 接続を試みる。
                    if self.connection.is_some() {
                        debug!("Already connected");
                        continue;
                    }

                    // middle-adpterが生成した名前付きパイプを開く
                    // (クライアント側は普通にファイルとして開くことができる)
                    // また、パイプを読み込み用と書き込み用に複製する
                    // (同一のパイプを指すオブジェクトを2つ作るということ。
                    //  Rustの所有権ルールのため、2つのスレッドからパイプにアクセスするためにはパイプへの参照が2つ必要となる)

                    debug!("[connection] クライアント側のパイプを開く");
                    let mut in_stream = fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(r"\\.\pipe\hdg-pipe")
                        .expect("open read pipe");
                    let out_stream = in_stream.try_clone().expect("duplicate pipe");

                    // 受信したメッセージを処理するためのワーカースレッドを建てる。
                    let app_sender = self.app_sender.clone();
                    let join_handle = thread::spawn(move || {
                        let mut r =
                            dac::DebugAdapterReader::new(io::BufReader::new(&mut in_stream));
                        let mut buf = Vec::new();
                        loop {
                            debug!("[dap-reader] recv");
                            if !r.recv(&mut buf) {
                                break;
                            }

                            let msg = match serde_json::from_slice::<dap::Msg>(&buf) {
                                Err(err) => {
                                    error!("[connection(2)] {:?}", err);
                                    continue;
                                }
                                Ok(msg) => msg,
                            };

                            app_sender.send(app::Action::AfterRequestReceived(msg));
                        }

                        info!("[connection] DAR 終了");
                    });

                    self.connection = Some((out_stream, join_handle));
                    self.app_sender.send(app::Action::AfterConnected);
                }
                Ok(Action::Send(msg)) => {
                    // 送信要求が来たとき: 接続が確立していたら送信する。
                    let mut stream = match self.connection {
                        None => {
                            warn!("送信 失敗 接続が確立していません");
                            continue;
                        }
                        Some((ref mut stream, _)) => stream,
                    };

                    dac::DebugAdapterWriter::new(&mut stream).write(&msg);
                }
                Err(err) => {
                    error!("[connection::Worker] {:?}", err);
                    break;
                }
            }
        }

        if let Some((stream, _)) = self.connection.take() {
            // stream.shutdown(net::Shutdown::Both).unwrap();
            drop(stream);

            // NOTE: なぜか停止しないので join しない。
            // join_handle.join().unwrap();
        }

        info!("[connection] 終了");
    }
}
