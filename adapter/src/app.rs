use connection;
use hsprt;
use hspsdk;
use logger;
use std;
use std::sync::mpsc;
use std::thread;
use ws;

/// VSCode 側のデバッグアダプターから送られてくるメッセージ。
/// E.g `{"type": "pause"}`
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum DebugRequest {
    /// 再開
    Continue,
    /// ステップオーバー (次行の実行)
    Next,
    /// 中断
    Pause,
    /// グローバル変数の一覧の要求
    Globals,
}

/// VSCode 側のデバッグアダプターに送るメッセージ。
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum DebugResponse {
    Continue,
    Stop { file: String, line: i32 },
    Globals { vars: Vec<Var> },
}

#[derive(Serialize, Clone, Debug)]
#[allow(non_snake_case)]
pub(crate) struct Var {
    pub name: String,
    pub value: String,
    pub variablesReference: i32,
}

/// `Worker` が扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// VSCode からメッセージが来たとき。(中断ボタンが押されたときなど。)
    AfterDebugRequestReceived(String),
    /// VSCode 側にメッセージを送信する。(assert で停止したときなど。)
    DebugEvent(DebugResponse),
    /// assert で停止したとき。
    EventStop(String, i32),
    /// VSCode との接続が確立したとき。
    AfterWebSocketConnected(ws::Sender),
}

/// `Worker` に処理を依頼するもの。
#[derive(Clone, Debug)]
pub(crate) struct Sender {
    sender: mpsc::Sender<Action>,
}

impl Sender {
    pub(crate) fn send(&self, action: Action) {
        self.sender
            .send(action)
            .map_err(|e| logger::log_error(&e))
            .ok();
    }
}

/// HSP ランタイムと VSCode の仲介を行う。
pub(crate) struct Worker<D> {
    app_sender: Sender,
    request_receiver: mpsc::Receiver<Action>,
    connection_sender: connection::Sender,
    ws_sender: Option<ws::Sender>,
    d: D,
}

impl<D: hsprt::HspDebug> Worker<D> {
    pub fn new(d: D) -> Self
    where
        D: Send + 'static,
    {
        let (sender, request_receiver) = mpsc::channel::<Action>();
        let app_sender = Sender { sender };

        let mut connection_worker = connection::Worker::new(app_sender.clone());
        let connection_sender = connection_worker.sender();
        thread::spawn(move || connection_worker.run());

        Worker {
            app_sender,
            request_receiver,
            connection_sender,
            ws_sender: None,
            d,
        }
    }

    pub fn sender(&self) -> Sender {
        self.app_sender.clone()
    }

    fn send(&mut self, action: Action) {
        self.app_sender.send(action);
    }

    pub fn run(mut self) {
        self.connection_sender.send(connection::Action::Connect);

        loop {
            match self.request_receiver.recv() {
                Ok(action) => {
                    self.handle(action);
                    continue;
                }
                Err(err) => {
                    logger::log_error(&err);
                    break;
                }
            }
        }
    }

    fn handle(&mut self, action: Action) {
        logger::log(&format!("[App] {:?}", action));
        match action {
            Action::AfterDebugRequestReceived(json) => match serde_json::from_str(&json) {
                Err(err) => {
                    logger::log("  不明なメッセージ");
                }
                Ok(DebugRequest::Continue) => {
                    self.d.set_mode(hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode);
                    self.send(Action::DebugEvent(DebugResponse::Continue));
                }
                Ok(DebugRequest::Pause) => {
                    self.d.set_mode(hspsdk::HSPDEBUG_STOP as hspsdk::DebugMode);
                    self.send(Action::DebugEvent(DebugResponse::Stop {
                        file: "main.hsp".to_owned(),
                        line: 6,
                    }));
                }
                Ok(DebugRequest::Next) => {
                    self.d
                        .set_mode(hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode);
                }
                Ok(DebugRequest::Globals) => {
                    self.d.get_globals();
                }
            },
            Action::EventStop(file, line) => {
                logger::log("送信 break");

                self.send(Action::DebugEvent(DebugResponse::Stop { file: file, line }));
            }
            Action::DebugEvent(response) => {
                let ws_sender = match self.ws_sender.as_ref() {
                    None => {
                        logger::log("WebSocket が接続を確立していないのでイベントを送信できませんでした。");
                        return;
                    }
                    Some(ws_sender) => ws_sender,
                };
                let message = match serde_json::to_string(&response) {
                    Err(e) => {
                        logger::log_error(&e);
                        return;
                    }
                    Ok(message) => message,
                };
                logger::log(&format!("送信 {:?}", message));

                ws_sender
                    .send(ws::Message::Text(message))
                    .map_err(|e| logger::log_error(&e))
                    .ok();
            }
            Action::AfterWebSocketConnected(ws_sender) => {
                self.ws_sender = Some(ws_sender);
            }
        }
    }
}
