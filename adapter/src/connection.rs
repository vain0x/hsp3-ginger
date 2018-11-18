//! VSCode 側のデバッガーアダプターと通信する。

#![allow(unused_imports)]

use app;
use helpers::failwith;
use hsprt;
use hspsdk;
use logger;
use std;
use std::sync::mpsc;
use std::{mem, sync, thread, time};
use ws;

/// デバッガーから VSCode に送るメッセージ。
pub(crate) enum DebugEvent {
    Stop,
}

/// コネクションワーカーが扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Connect,
    AfterConnectionFailed,
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
}

impl Worker {
    pub fn build(app_sender: app::Sender) -> Self {
        let (sender, receiver) = mpsc::channel::<Action>();
        Worker {
            app_sender,
            connection_sender: Sender { sender },
            receiver,
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
                    WebSocketHandler::build(self.sender(), self.app_sender.clone()).try_connect();
                }
                Ok(Action::AfterConnectionFailed) => {
                    // 接続に失敗したとき: 3秒待って再試行する。
                    thread::sleep(time::Duration::from_secs(3));
                    self.connection_sender.send(Action::Connect);
                }
                Err(err) => {
                    logger::log_error(&err);
                    break;
                }
            }
        }
    }
}

/// WebSocket 経由で VSCode から送られてくるメッセージを処理するもの。
#[derive(Clone)]
struct WebSocketHandler {
    app_sender: app::Sender,
    connection_sender: Sender,
}

impl WebSocketHandler {
    fn build(connection_sender: Sender, app_sender: app::Sender) -> Self {
        WebSocketHandler {
            connection_sender,
            app_sender,
        }
    }

    fn try_connect(&self) {
        // NOTE: 接続に成功したら、接続が切れるまで connect 関数は終了しない。
        let result = ws::connect("ws://localhost:8089/", |out: ws::Sender| {
            logger::log("[WS] 接続");

            // 接続の確立を通知する。メッセージを送信するためのオブジェクトを外部に送る。
            self.app_sender
                .send(app::Action::AfterWebSocketConnected(out));

            // `ws::Handler` を返す。
            self.clone()
        });

        match result {
            Ok(()) => {}
            Err(_) => {
                logger::log("[WS] 接続 失敗");
                self.connection_sender.send(Action::AfterConnectionFailed);
            }
        }
    }
}

impl ws::Handler for WebSocketHandler {
    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        match message {
            ws::Message::Binary(_) => {
                logger::log("[WS] 受信 失敗 バイナリ");
                Ok(())
            }
            ws::Message::Text(json) => {
                logger::log(&format!("[WS] 受信 {}", json));
                self.app_sender
                    .send(app::Action::AfterDebugRequestReceived(json));
                Ok(())
            }
        }
    }
}
