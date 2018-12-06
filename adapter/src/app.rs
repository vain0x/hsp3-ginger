use connection;
use debug_adapter_protocol as dap;
use hsp_ext;
use hsprt;
use hspsdk;
use logger;
use std;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

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
}

/// Variables reference. VSCode が変数や変数要素を指し示すために使う整数値。
pub(crate) type VarRef = i64;

impl VarPath {
    fn to_var_ref(&self) -> VarRef {
        match self {
            VarPath::Globals => 1,
        }
    }

    fn from_var_ref(r: VarRef) -> Option<Self> {
        match r {
            1 => Some(VarPath::Globals),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeState {
    file: Option<String>,
    line: i32,
    stopped: bool,
}

/// `Worker` が扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// VSCode との接続が確立したとき。
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
pub(crate) struct Worker {
    request_receiver: mpsc::Receiver<Action>,
    connection_sender: Option<connection::Sender>,
    hsprt_sender: Option<hsprt::Sender>,
    args: Option<dap::LaunchRequestArgs>,
    state: RuntimeState,
    debug_info: Option<hsp_ext::debug_info::DebugInfo<hsp_ext::debug_info::HspConstantMap>>,
    source_map: Option<hsp_ext::source_map::SourceMap>,
    #[allow(unused)]
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(hsprt_sender: hsprt::Sender) -> (Self, Sender) {
        let (sender, request_receiver) = mpsc::channel::<Action>();
        let app_sender = Sender { sender };

        let (connection_worker, connection_sender) = connection::Worker::new(app_sender.clone());
        let join_handle = thread::Builder::new()
            .name("connection_worker".into())
            .spawn(move || connection_worker.run())
            .unwrap();

        let worker = Worker {
            request_receiver,
            connection_sender: Some(connection_sender),
            hsprt_sender: Some(hsprt_sender),
            args: None,
            state: RuntimeState {
                file: None,
                line: 1,
                stopped: false,
            },
            debug_info: None,
            source_map: None,
            join_handle: Some(join_handle),
        };

        (worker, app_sender)
    }

    pub fn run(mut self) {
        self.connection_sender
            .as_ref()
            .unwrap()
            .send(connection::Action::Connect);

        loop {
            match self.request_receiver.recv() {
                Ok(action @ Action::BeforeTerminating) => {
                    self.handle(action);
                    break;
                }
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

        logger::log("[app] 終了");
    }

    /// HSP ランタイムが次に中断しているときにアクションが実行されるように予約する。
    /// すでに停止しているときは即座に実行されるように、メッセージを送る。
    fn send_to_hsprt(&self, action: hsprt::Action) {
        if let Some(sender) = self.hsprt_sender.as_ref() {
            sender.send(action, self.state.stopped);
        }
    }

    fn send_response(&mut self, request_seq: i64, response: dap::Response) {
        if let Some(sender) = self.connection_sender.as_ref() {
            sender.send(connection::Action::Send(dap::Msg::Response {
                request_seq,
                success: true,
                e: response,
            }));
        }
    }

    fn send_event(&mut self, event: dap::Event) {
        if let Some(sender) = self.connection_sender.as_ref() {
            sender.send(connection::Action::Send(dap::Msg::Event { e: event }));
        }
    }

    fn on_request(&mut self, seq: i64, request: dap::Request) {
        match request {
            dap::Request::Options { args } => {
                self.args = Some(args);
                self.load_source_map();
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
                // self.d.terminate();
            }
        }
    }

    fn load_source_map(&mut self) {
        if self.source_map.is_some() {
            return;
        }

        let debug_info = match self.debug_info {
            None => return,
            Some(ref debug_info) => debug_info,
        };

        let args = match self.args {
            None => return,
            Some(ref args) => args,
        };
        let root = PathBuf::from(&args.root);

        let mut source_map = hsp_ext::source_map::SourceMap::new(&root);
        let file_names = debug_info.file_names();

        source_map.add_search_path(PathBuf::from(&args.program).parent());
        source_map.add_file_names(
            &file_names
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<&str>>(),
        );
        logger::log(&format!("{:?}", source_map));

        self.source_map = Some(source_map);
    }

    /// ファイル名を絶対パスにする。
    /// FIXME: common 以下や 無修飾 include パスに対応する。
    fn resolve_file_path(&self, file_name: String) -> Option<String> {
        if file_name == "???" {
            return None;
        }

        let source_map = self.source_map.as_ref()?;
        let full_path = source_map.resolve_file_name(&file_name)?;
        Some(full_path.to_str()?.to_owned())
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

                self.state = RuntimeState {
                    file,
                    line,
                    stopped: true,
                };
                self.send_event(dap::Event::Stopped {
                    reason: "pause".to_owned(),
                    thread_id: MAIN_THREAD_ID,
                });
            }
            Action::AfterConnected => {
                self.send_event(dap::Event::Initialized);
            }
            Action::BeforeTerminating => {
                self.send_event(dap::Event::Terminated { restart: false });

                // サブワーカーを捨てる。
                self.hsprt_sender.take();
                self.connection_sender.take();

                if let Some(_) = self.join_handle.take() {
                    // NOTE: なぜか終了しないので join しない。
                    // join_handle.join().unwrap();
                }
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
