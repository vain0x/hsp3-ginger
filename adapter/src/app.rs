use connection;
use debug_adapter_protocol as dap;
use hsprt;
use hspsdk;
use logger;
use std;
use std::sync::mpsc;
use std::thread;

const MAIN_THREAD_ID: i64 = 1;
const MAIN_THREAD_NAME: &'static str = "main";

// グローバル変数からなるスコープの変数参照Id
const GLOBAL_SCOPE_REF: i64 = 1;

fn threads() -> Vec<dap::Thread> {
    vec![dap::Thread {
        id: MAIN_THREAD_ID,
        name: MAIN_THREAD_NAME.to_owned(),
    }]
}

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
    Globals {
        seq: i64,
        variables: Vec<dap::Variable>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeState {
    file: Option<String>,
    line: i32,
}

/// `Worker` が扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// VSCode との接続が確立したとき。
    AfterConnected,
    /// VSCode からリクエストが来たとき。
    AfterRequestReceived(dap::Msg),
    /// VSCode 側にメッセージを送信する。(assert で停止したときなど。)
    DebugEvent(DebugResponse),
    /// assert で停止したとき。
    AfterStopped(String, i32),
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
    state: RuntimeState,
    args: Option<dap::LaunchRequestArgs>,
    d: D,
}

impl<D: hsprt::HspDebug> Worker<D> {
    pub fn new(d: D) -> Self {
        let (sender, request_receiver) = mpsc::channel::<Action>();
        let app_sender = Sender { sender };

        let connection_worker = connection::Worker::new(app_sender.clone());
        let connection_sender = connection_worker.sender();
        thread::spawn(move || connection_worker.run());

        Worker {
            app_sender,
            request_receiver,
            connection_sender,
            state: RuntimeState {
                file: None,
                line: 1,
            },
            args: None,
            d,
        }
    }

    pub fn sender(&self) -> Sender {
        self.app_sender.clone()
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

    fn send_response(&mut self, request_seq: i64, response: dap::Response) {
        self.connection_sender
            .send(connection::Action::Send(dap::Msg::Response {
                request_seq,
                success: true,
                e: response,
            }));
    }

    fn send_event(&mut self, event: dap::Event) {
        self.connection_sender
            .send(connection::Action::Send(dap::Msg::Event { e: event }));
    }

    fn on_request(&mut self, seq: i64, request: dap::Request) {
        match request {
            dap::Request::Options { args } => {
                self.args = Some(args);
                // NOTE: VSCode からのリクエストではないのでレスポンス不要。
            }
            dap::Request::SetExceptionBreakpoints { .. } => {
                self.send_response(seq, dap::Response::SetExceptionBreakpoints);
            }
            dap::Request::ConfigurationDone => {
                self.send_response(seq, dap::Response::ConfigurationDone);
            }
            dap::Request::Threads => {
                self.send_response(seq, dap::Response::Threads { threads: threads() })
            }
            dap::Request::StackTrace { .. } => {
                let stack_frames = vec![dap::StackFrame {
                    id: 1,
                    name: "main".to_owned(),
                    line: std::cmp::max(1, self.state.line) as usize,
                    source: dap::Source {
                        name: "main.hsp".to_owned(),
                        path: self.state.file.to_owned(),
                    },
                }];
                self.send_response(seq, dap::Response::StackTrace { stack_frames });
            }
            dap::Request::Scopes { .. } => {
                let scopes = vec![dap::Scope {
                    name: "グローバル".to_owned(),
                    variables_reference: GLOBAL_SCOPE_REF,
                    expensive: true,
                }];
                self.send_response(seq, dap::Response::Scopes { scopes });
            }
            dap::Request::Variables {
                variables_reference,
            } => {
                if variables_reference == GLOBAL_SCOPE_REF {
                    self.d.get_globals(seq);
                }
            }
            dap::Request::Pause { .. } => {
                self.d.set_mode(hspsdk::HSPDEBUG_STOP as hspsdk::DebugMode);
                self.send_response(
                    seq,
                    dap::Response::Pause {
                        thread_id: MAIN_THREAD_ID,
                    },
                )
            }
            dap::Request::Continue { .. } => {
                self.d.set_mode(hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode);
                self.send_response(seq, dap::Response::Continue);
                self.send_event(dap::Event::Continued {
                    all_threads_continued: true,
                });
            }
            dap::Request::Next { .. } => {
                self.d
                    .set_mode(hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode);
                self.send_response(seq, dap::Response::Next);
            }
            dap::Request::StepIn { .. } => {
                self.d
                    .set_mode(hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode);
                self.send_response(seq, dap::Response::StepIn);
            }
            dap::Request::StepOut { .. } => {
                self.d
                    .set_mode(hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode);
                self.send_response(seq, dap::Response::StepOut);
            }
            dap::Request::Disconnect { .. } => {
                // self.d.terminate();
            }
        }
    }

    /// ファイル名を絶対パスにする。
    /// FIXME: common 以下や 無修飾 include パスに対応する。
    fn resolve_file_path(&self, file_name: String) -> Option<String> {
        if file_name == "???" {
            return None;
        }

        let args = self.args.as_ref()?;
        let file_path = std::path::PathBuf::from(args.cwd.to_owned())
            .join(&file_name)
            .canonicalize()
            .ok()?;

        if !std::fs::metadata(&file_path).is_ok() {
            return None;
        }

        Some(file_path.to_str()?.to_owned())
    }

    fn handle(&mut self, action: Action) {
        logger::log(&format!("[App] {:?}", action));
        match action {
            Action::AfterRequestReceived(dap::Msg::Request { seq, e }) => {
                self.on_request(seq, e);
            }
            Action::AfterRequestReceived(_) => {
                logger::log("受信 リクエストではない DAP メッセージを無視");
            }
            Action::AfterStopped(file, line) => {
                logger::log("送信 中断");

                let file = self.resolve_file_path(file);

                self.state = RuntimeState { file, line };
                self.send_event(dap::Event::Stopped {
                    reason: "pause".to_owned(),
                    thread_id: MAIN_THREAD_ID,
                });
            }
            Action::DebugEvent(response) => match response {
                DebugResponse::Globals { seq, variables } => {
                    self.send_response(seq, dap::Response::Variables { variables });
                }
            },
            Action::AfterConnected => {
                self.send_event(dap::Event::Initialized);
            }
        }
    }
}
