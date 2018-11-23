//! VSCode 側のデバッガーアダプターと通信する。

#![allow(unused_imports)]

use app;
use debug_adapter_connection as dac;
use debug_adapter_protocol as dap;
use hsprt;
use hspsdk;
use logger;
use std;
use std::sync::mpsc;
use std::{fmt, io, mem, net, sync, thread, time};
use ws;

#[derive(Clone, Debug)]
pub(crate) struct MyLogger;

impl dac::Logger for MyLogger {
    fn log(&self, args: fmt::Arguments) {
        logger::log(&fmt::format(args));
    }
}

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
            .map_err(|err| logger::log_error(&err))
            .ok();
    }
}

/// コネクションワーカー。
/// VSCode 側のデバッガーアダプターが立てている WebSocket サーバーに接続して双方向通信を行う。
pub(crate) struct Worker {
    app_sender: app::Sender,
    connection_sender: Sender,
    receiver: mpsc::Receiver<Action>,
    connection: Option<(net::TcpStream, mpsc::Sender<dap::Msg>)>,
}

impl Worker {
    pub fn new(app_sender: app::Sender) -> Self {
        let (sender, receiver) = mpsc::channel::<Action>();
        Worker {
            app_sender,
            connection_sender: Sender { sender },
            receiver,
            connection: None,
        }
    }

    pub fn sender(&self) -> Sender {
        self.connection_sender.clone()
    }

    pub fn run(mut self) {
        loop {
            match self.receiver.recv() {
                Ok(Action::Connect) => {
                    // 接続要求が来たとき: 接続を試みる。
                    if self.connection.is_some() {
                        continue;
                    }

                    let port = 57676;
                    let stream = match net::TcpStream::connect(("127.0.0.1", port)) {
                        Ok(stream) => stream,
                        Err(err) => {
                            logger::log_error(&err);
                            continue;
                        }
                    };
                    let in_stream = stream.try_clone().unwrap();

                    // 受信したメッセージを処理するためのワーカースレッドを建てる。
                    let (tx, _) = mpsc::channel();
                    let app_sender = self.app_sender.clone();
                    thread::spawn(move || {
                        let mut r =
                            dac::DebugAdapterReader::new(io::BufReader::new(in_stream), MyLogger);
                        let mut buf = Vec::new();
                        loop {
                            if !r.recv(&mut buf) {
                                break;
                            }

                            let msg = match serde_json::from_slice::<dap::Msg>(&buf) {
                                Err(err) => {
                                    logger::log_error(&err);
                                    continue;
                                }
                                Ok(msg) => msg,
                            };

                            app_sender.send(app::Action::AfterRequestReceived(msg));
                        }
                    });

                    self.connection = Some((stream, tx));
                    self.app_sender.send(app::Action::AfterConnected);
                }
                Ok(Action::Send(msg)) => {
                    // 送信要求が来たとき: 接続が確立していたら送信する。
                    let stream = match self.connection {
                        None => {
                            logger::log("送信 失敗 接続が確立していません");
                            continue;
                        }
                        Some((ref stream, _)) => stream,
                    };

                    dac::DebugAdapterWriter::new(stream, MyLogger).write(&msg);
                }
                Err(err) => {
                    logger::log_error(&err);
                    break;
                }
            }
        }
    }
}
