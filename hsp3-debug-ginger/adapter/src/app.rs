use crate::{connection, hsp_ext, hsprt, hspsdk};
use log::{debug, error, info, warn};
use shared::debug_adapter_protocol as dap;
use std::{self, mem, path::PathBuf, sync::mpsc};

const MAIN_THREAD_ID: i64 = 1;
const MAIN_THREAD_NAME: &'static str = "main";

fn threads() -> Vec<dap::Thread> {
    vec![dap::Thread {
        id: MAIN_THREAD_ID,
        name: MAIN_THREAD_NAME.to_owned(),
    }]
}

/// グローバル変数からなるスコープの変数参照Id
const GLOBAL_SCOPE_REF: i64 = 1;

/// HSP の変数や変数の要素、あるいは変数をまとめるもの (モジュールなど) を指し示すもの。
#[derive(Clone, Debug)]
pub(crate) enum VarPath {
    Globals,
    Static(usize),
}

/// Variables reference. VSCode が変数や変数要素を指し示すために使う整数値。
pub(crate) type VarRef = i64;

impl VarPath {
    pub fn to_var_ref(&self) -> VarRef {
        match *self {
            VarPath::Globals => 1,
            VarPath::Static(i) => 2 + i as i64,
        }
    }

    pub fn from_var_ref(r: VarRef) -> Option<Self> {
        match r {
            1 => Some(VarPath::Globals),
            i if i >= 2 => Some(VarPath::Static((i - 2) as usize)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeState {
    file_name: Option<String>,
    file_path: Option<String>,
    line: i32,
    stopped: bool,
}

/// `Worker` が扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// アダプターとの接続が確立したとき。
    AfterConnected,
    /// VSCode からリクエストが来たとき。
    AfterRequestReceived(dap::Msg),
    /// assert で停止したとき。
    AfterStopped(String, i32),
    /// HSP ランタイムが終了する直前。
    BeforeTerminating,
    AfterDebugInfoLoaded(hsp_ext::debug_info::DebugInfo<hsp_ext::debug_info::HspConstantMap>),
    AfterGetVar {
        seq: i64,
        variables: Vec<dap::Variable>,
    },
}

/// `Worker` に処理を依頼するもの。
#[derive(Clone, Debug)]
pub(crate) struct Sender {
    sender: mpsc::SyncSender<Action>,
}

impl Sender {
    pub(crate) fn send(&self, action: Action) {
        self.sender
            .send(action)
            .unwrap_or_else(|err| error!("[app::Sender] {:?}", err));
    }
}

/// HSP ランタイムと VSCode の仲介を行う。
pub(crate) struct Worker {
    rx: mpsc::Receiver<Action>,
    finish_tx: mpsc::SyncSender<()>,
    connection_sender: Option<connection::Sender>,
    hsprt_sender: Option<hsprt::Sender>,
    is_connected: bool,
    launch_args: Option<dap::LaunchRequestArgs>,
    state: RuntimeState,
    debug_info: Option<hsp_ext::debug_info::DebugInfo<hsp_ext::debug_info::HspConstantMap>>,
    source_map: Option<hsp_ext::source_map::SourceMap>,
    did_send_initialized_event: bool,
    // #[allow(unused)]
    // join_handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(hsprt_sender: hsprt::Sender) -> (Self, Sender, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::sync_channel::<Action>(8);
        let (finish_tx, finish_rx) = mpsc::sync_channel(1);

        let sender = Sender { sender: tx };

        let worker = Worker {
            rx,
            finish_tx,
            connection_sender: None,
            hsprt_sender: Some(hsprt_sender),
            is_connected: false,
            launch_args: None,
            state: RuntimeState {
                file_path: None,
                file_name: None,
                line: 1,
                stopped: false,
            },
            debug_info: None,
            source_map: None,
            did_send_initialized_event: false,
            // join_handle: Some(join_handle),
        };

        (worker, sender, finish_rx)
    }

    pub fn set_connection_sender(&mut self, connection_sender: connection::Sender) {
        self.connection_sender = Some(connection_sender);
    }

    fn is_launch_response_sent(&self) -> bool {
        self.launch_args.is_some()
    }

    pub fn run(mut self) {
        debug!("[app] 開始");

        loop {
            match self.rx.recv() {
                Ok(action @ Action::BeforeTerminating) => {
                    self.handle(action);
                    break;
                }
                Ok(action) => {
                    self.handle(action);
                    continue;
                }
                Err(err) => {
                    error!("[app] {:?}", err);
                    break;
                }
            }
        }

        info!("[app] 終了");
        self.finish_tx.send(()).unwrap_or_else(|_| {
            error!("[app] finish_tx.send");
        });
    }

    /// HSP ランタイムが次に中断しているときにアクションが実行されるように予約する。
    /// すでに停止しているときは即座に実行されるように、メッセージを送る。
    fn send_to_hsprt(&self, action: hsprt::Action) {
        let sender = match &self.hsprt_sender {
            Some(it) => it,
            None => {
                debug!("[app] send_to_hsprt: sender is none");
                return;
            }
        };
        sender.send(action, self.state.stopped);
    }

    fn send_response(&mut self, request_seq: i64, response: dap::Response) {
        let sender = match &self.connection_sender {
            Some(it) => it,
            None => {
                debug!("[app] send_response: sender is none");
                return;
            }
        };
        sender.send(connection::Action::Send(dap::Msg::Response {
            request_seq,
            success: true,
            e: response,
        }));
    }

    fn send_response_failure(&mut self, request_seq: i64, response: dap::Response) {
        let sender = match &self.connection_sender {
            Some(it) => it,
            None => {
                debug!("[app] send_response_failure: sender is none");
                return;
            }
        };
        sender.send(connection::Action::Send(dap::Msg::Response {
            request_seq,
            success: false,
            e: response,
        }));
    }

    fn send_event(&self, event: dap::Event) {
        let sender = match &self.connection_sender {
            Some(it) => it,
            None => {
                debug!("[app] send_event: sender is none");
                return;
            }
        };
        sender.send(connection::Action::Send(dap::Msg::Event { e: event }));
    }

    fn send_pause_event(&self) {
        if self.state.stopped && self.is_launch_response_sent() {
            self.send_event(dap::Event::Stopped {
                reason: "pause".to_owned(),
                thread_id: MAIN_THREAD_ID,
            });
        } else {
            debug!("[app] send_pause_event skipped");
        }
    }

    fn on_request(&mut self, seq: i64, request: dap::Request) {
        match request {
            dap::Request::Launch(args) => {
                self.launch_args = Some(args);
                self.load_source_map();
                self.send_response(seq, dap::Response::Launch);

                if !mem::replace(&mut self.did_send_initialized_event, true) {
                    self.send_event(dap::Event::Initialized);
                }
            }
            dap::Request::SetExceptionBreakpoints { .. } => {
                self.send_response(seq, dap::Response::SetExceptionBreakpoints);
                self.send_pause_event();
            }
            dap::Request::ConfigurationDone => {
                self.send_response(seq, dap::Response::ConfigurationDone);
            }
            dap::Request::Threads => {
                self.send_response(seq, dap::Response::Threads { threads: threads() })
            }
            dap::Request::Source { source } => {
                match source.and_then(|source| Some(std::fs::read_to_string(source.path?).ok()?)) {
                    Some(content) => self.send_response(seq, dap::Response::Source { content }),
                    None => self.send_response_failure(
                        seq,
                        dap::Response::Source {
                            content: "".to_owned(),
                        },
                    ),
                }
            }
            dap::Request::StackTrace { .. } => {
                if self.state.file_path.is_none() {
                    let file_path = self
                        .state
                        .file_name
                        .as_ref()
                        .and_then(|file_name| self.resolve_file_path(file_name));
                    self.state.file_path = file_path;
                }

                let stack_frames = vec![dap::StackFrame {
                    id: 1,
                    name: "main".to_owned(),
                    line: std::cmp::max(1, self.state.line) as usize,
                    source: dap::Source {
                        name: "main".to_owned(),
                        path: self.state.file_path.to_owned(),
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
                if let Some(var_path) = VarPath::from_var_ref(variables_reference) {
                    self.send_to_hsprt(hsprt::Action::GetVar { seq, var_path });
                }
            }
            dap::Request::Pause { .. } => {
                self.send_to_hsprt(hsprt::Action::SetMode(
                    hspsdk::HSPDEBUG_STOP as hspsdk::DebugMode,
                ));
                self.send_response(
                    seq,
                    dap::Response::Pause {
                        thread_id: MAIN_THREAD_ID,
                    },
                );
            }
            dap::Request::Continue { .. } => {
                self.send_to_hsprt(hsprt::Action::SetMode(
                    hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode,
                ));
                self.send_response(seq, dap::Response::Continue);
                self.send_event(dap::Event::Continued {
                    all_threads_continued: true,
                });
                self.state.stopped = false;
            }
            dap::Request::Next { .. } => {
                self.send_to_hsprt(hsprt::Action::SetMode(
                    hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode,
                ));
                self.send_response(seq, dap::Response::Next);
            }
            dap::Request::StepIn { .. } => {
                self.send_to_hsprt(hsprt::Action::SetMode(
                    hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode,
                ));
                self.send_response(seq, dap::Response::StepIn);
            }
            dap::Request::StepOut { .. } => {
                self.send_to_hsprt(hsprt::Action::SetMode(
                    hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode,
                ));
                self.send_response(seq, dap::Response::StepOut);
            }
            dap::Request::Disconnect { .. } => {
                self.send_to_hsprt(hsprt::Action::Disconnect);
            }
        }
    }

    fn load_source_map(&mut self) {
        if self.source_map.is_some() {
            return;
        }

        let debug_info = match &self.debug_info {
            Some(it) => it,
            None => return,
        };

        let launch_args = match &self.launch_args {
            Some(it) => it,
            None => return,
        };

        let hsp3_root = PathBuf::from(&launch_args.root);

        let mut source_map = hsp_ext::source_map::SourceMap::new(&hsp3_root);
        let file_names = debug_info.file_names();

        source_map.add_search_path(PathBuf::from(&launch_args.program).parent());
        source_map.add_file_names(
            &file_names
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<&str>>(),
        );

        self.source_map = Some(source_map);
    }

    /// ファイル名を絶対パスにする。
    /// FIXME: common 以下や 無修飾 include パスに対応する。
    fn resolve_file_path(&self, file_name: &String) -> Option<String> {
        if file_name == "???" {
            return None;
        }

        let source_map = self.source_map.as_ref()?;
        let full_path = source_map.resolve_file_name(file_name)?;
        Some(full_path.to_str()?.to_owned())
    }

    fn handle(&mut self, action: Action) {
        debug!("[app] {:?}", action);

        match action {
            Action::AfterRequestReceived(dap::Msg::Request { seq, e }) => {
                self.on_request(seq, e);
            }
            Action::AfterRequestReceived(_) => {
                warn!("[app] リクエストではない DAP メッセージを無視");
            }
            Action::AfterStopped(file_name, line) => {
                let file_path = self.resolve_file_path(&file_name);

                self.state = RuntimeState {
                    file_path,
                    file_name: Some(file_name),
                    line,
                    stopped: true,
                };
                self.send_pause_event();
            }
            Action::AfterConnected => {
                self.is_connected = true;
            }
            Action::BeforeTerminating => {
                self.send_event(dap::Event::Terminated { restart: false });

                // サブワーカーを捨てる。
                // self.hsprt_sender.take();
                // self.connection_sender.take();

                // if let Some(_) = self.join_handle.take() {
                //     // NOTE: なぜか終了しないので join しない。
                //     // join_handle.join().unwrap();
                // }
            }
            Action::AfterDebugInfoLoaded(debug_info) => {
                self.debug_info = Some(debug_info);
                self.load_source_map();
            }
            Action::AfterGetVar { seq, variables } => {
                self.send_response(seq, dap::Response::Variables { variables });
            }
        }
    }
}
