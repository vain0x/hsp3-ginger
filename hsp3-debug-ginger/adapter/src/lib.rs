//! デバッガーのエントリーポイント。

mod app;
mod connection;
mod helpers;
mod hsp_ext;
mod hsprt;
mod hspsdk;

mod util {
    pub(crate) mod single_thread;
}

use crate::{hsprt::*, util::single_thread::SingleThread};
use log::{debug, info, warn};
use once_cell::sync::OnceCell;
use shared::{debug_adapter_protocol as dap, file_logger::FileLogger};
use std::{
    path::PathBuf,
    ptr,
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

#[cfg(windows)]
use winapi::shared::minwindef::*;

static GLOBALS: OnceCell<Mutex<Option<SingleThread<Globals>>>> = OnceCell::new();

type HspMsgFunc = Option<unsafe extern "C" fn(*mut hspsdk::HSPCTX)>;

/// グローバル変数をまとめたもの。
/// `debug_notice` などの関数に状態をもたせるために使う。
pub(crate) struct Globals {
    app_sender: app::Sender,
    hsprt_receiver: mpsc::Receiver<Action>,
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    default_msgfunc: HspMsgFunc,
    #[allow(unused)]
    join_handles: Vec<thread::JoinHandle<()>>,
}

impl Globals {
    /// 初期化処理を行い、各グローバル変数の初期値を設定して `Globals` を構築する。
    fn new(hsp_debug: *mut hspsdk::HSP3DEBUG) -> Self {
        // msgfunc に操作を送信するチャネルを生成する。
        let (sender, hsprt_receiver) = mpsc::channel();
        let (notice_sender, _notice_receiver) = mpsc::channel();

        let hsprt_sender = Sender::new(sender, notice_sender);

        let j1 = thread::spawn(move || {
            // HSP ランタイムが停止している状態で処理依頼が来るたびに notice_receiver が通知を受け取り、
            // そのたびにループが進行する。msgfunc に代わって処理を行う。
            // FIXME: ~ワーカースレッドで globals に触るのはとても危険なので同期化機構を使うべき。~
            // FIXME: メインスレッドにメッセージを送ってreceiveactionsを実行させる
            // for _ in notice_receiver {
            //     with_globals(|g| {
            //         g.receive_actions();
            //     });
            // }
            debug!("[notice] 終了");
        });

        // ワーカースレッドを起動する。
        let (app_worker, app_sender) = app::Worker::new(hsprt_sender);
        let j2 = thread::spawn(move || app_worker.run());

        let mut globals = Globals {
            app_sender,
            hsprt_receiver,
            hsp_debug,
            default_msgfunc: None,
            join_handles: vec![j1, j2],
        };

        unsafe { globals.hook_msgfunc() };

        globals.load_debug_info();

        globals
    }

    /// HSP のメッセージ関数をフックする。
    /// つまり、 wait/await などの際に `my_msgfunc` が呼ばれるようにする。
    /// 結果として返されるもとのメッセージ関数は `my_msgfunc` の中で呼び出す必要がある。
    unsafe fn hook_msgfunc(&mut self) {
        let default_msgfunc = self.hspctx().msgfunc;

        self.hspctx().msgfunc = Some(my_msgfunc);

        self.default_msgfunc = default_msgfunc;
    }

    fn hsp_debug<'a>(&'a self) -> &'a mut hspsdk::HSP3DEBUG {
        unsafe { &mut *self.hsp_debug }
    }

    fn hspctx<'a>(&'a self) -> &'a mut hspsdk::HSPCTX {
        let hspctx: *mut hspsdk::HSPCTX = self.hsp_debug().hspctx;
        unsafe { &mut *hspctx }
    }

    fn on_msgfunc_called(&mut self) {
        self.receive_actions();

        if let Some(default_msgfunc) = self.default_msgfunc {
            let hspctx = self.hspctx() as *mut hspsdk::HSPCTX;
            unsafe { default_msgfunc(hspctx) };
        }
    }

    /// メインスレッド上で実行すべき操作があれば、すべて実行する。なければ何もしない。
    fn receive_actions(&mut self) {
        while let Ok(action) = self.hsprt_receiver.try_recv() {
            self.do_action(action);
        }
    }

    /// `Action` で指定された操作を実行する。
    fn do_action(&mut self, action: Action) {
        match action {
            Action::SetMode(mode) => {
                self.do_set_mode(mode);
            }
            Action::GetVar { seq, var_path } => {
                self.do_get_var(seq, var_path);
            }
            Action::Disconnect => {
                self.do_disconnect();
            }
        }
    }

    // NOTE: 中断モードへの変更は HSP 側が wait/await で中断しているときに行わなければ無視されるので注意。
    fn do_set_mode(&self, mode: hspsdk::DebugMode) {
        let set = self.hsp_debug().dbg_set.unwrap();
        unsafe { set(mode) };
        unsafe { tap_all_windows() };
    }

    fn do_get_var(&mut self, seq: i64, var_path: app::VarPath) {
        match var_path {
            app::VarPath::Globals => self.do_get_globals(seq),
            app::VarPath::Static(i) => self.do_get_static(seq, i),
        }
    }

    fn static_var_metadata(&mut self, vi: usize) -> Option<(&'static str, bool, usize)> {
        let mut hspctx = hsp_ext::var::HspContext::from(self.hspctx());
        let pval = hspctx.static_vars().get_mut(vi)?;
        let ty = hsp_ext::var::Ty::from_flag(pval.flag as i32).name();
        let len = pval.len[1] as usize;
        let is_array = len > 1; // FIXME: 2次元配列は未対応
        Some((ty, is_array, len))
    }

    fn static_var_value(&mut self, vi: usize, i: usize) -> String {
        (|| {
            let mut hspctx = hsp_ext::var::HspContext::from(self.hspctx());
            let pval = hspctx.static_vars().get_mut(vi)?;
            let element = hspctx.var_element_ref(pval, i as hsp_ext::var::Aptr);
            Some(element.to_copy().into_string())
        })()
        .unwrap_or_else(|| "unknown".to_owned())
    }

    fn do_get_static(&mut self, seq: i64, vi: usize) {
        let variables = (|| {
            let (ty, _, len) = self.static_var_metadata(vi)?;

            let mut elements = vec![];
            for i in 0..len {
                let value = self.static_var_value(vi, i);
                elements.push(dap::Variable {
                    name: i.to_string(),
                    value,
                    ty: Some(ty.to_string()),
                    variables_reference: 0,
                    indexed_variables: None,
                })
            }
            Some(elements)
        })()
        .unwrap_or(vec![]);
        self.app_sender
            .send(app::Action::AfterGetVar { seq, variables });
    }

    fn do_get_globals(&mut self, seq: i64) {
        let var_names;
        {
            let d = self.hsp_debug();
            let get_varinf = d.get_varinf.unwrap();
            let dbg_close = d.dbg_close.unwrap();

            let p = unsafe { get_varinf(ptr::null_mut(), 0xFF) };
            var_names = helpers::string_from_hsp_str(p as *const u8);
            unsafe { dbg_close(p) };
        }
        let var_names = var_names.trim_end().split("\n").map(|s| s.trim_end());

        let mut variables = vec![];
        for (i, name) in var_names.enumerate() {
            let v = match self.static_var_metadata(i) {
                Some((ty, is_array, len)) if is_array => {
                    let variables_reference = app::VarPath::Static(i).to_var_ref();

                    dap::Variable {
                        name: name.to_owned(),
                        value: format!("count={}", len),
                        ty: Some(ty.to_owned()),
                        variables_reference,
                        indexed_variables: Some(len),
                    }
                }
                Some((ty, _, _)) => dap::Variable {
                    name: name.to_owned(),
                    value: self.static_var_value(i, 0),
                    ty: Some(ty.to_owned()),
                    variables_reference: 0,
                    indexed_variables: None,
                },
                None => dap::Variable {
                    name: name.to_owned(),
                    value: "unknown".to_owned(),
                    ty: None,
                    variables_reference: 0,
                    indexed_variables: None,
                },
            };
            variables.push(v);
        }

        self.app_sender
            .send(app::Action::AfterGetVar { seq, variables });
    }

    fn do_disconnect(&mut self) {
        let hspctx = self.hspctx();
        let put_error = hspctx.exinfo.HspFunc_puterror.unwrap();
        unsafe { put_error(hspsdk::HSPERROR_HSPERR_NONE) };
    }

    fn on_logmes_called(&mut self) {
        let message = helpers::string_from_hsp_str(self.hspctx().stmp as *const u8);
        info!("[logmes] {}", message);
    }

    /// assert などで停止したときに呼ばれる。
    fn on_stopped(&mut self) {
        let d = self.hsp_debug();

        let (file_name, line) = {
            // 実行位置 (ファイル名と行番号) の情報を更新する。
            let curinf = d.dbg_curinf.unwrap();
            unsafe { curinf() };

            let file_name = helpers::string_from_hsp_str(d.fname as *const u8);

            (file_name, d.line)
        };

        // 停止イベントを VSCode 側に通知する。
        self.app_sender
            .send(app::Action::AfterStopped(file_name, line));
    }

    // fn terminate(self) {
    //     let _join_handles = {
    //         let (app_sender, join_handles) = (self.app_sender, self.join_handles);
    //         app_sender.send(app::Action::BeforeTerminating);
    //         join_handles
    //     };

    //     // NOTE: なぜかスレッドが停止しない。
    //     // for j in join_handles {
    //     //     j.join().unwrap();
    //     // }
    //     thread::sleep(time::Duration::from_secs(3));
    // }

    fn load_debug_info(&self) {
        let debug_info = hsp_ext::debug_info::DebugInfo::parse_hspctx(self.hspctx());
        self.app_sender
            .send(app::Action::AfterDebugInfoLoaded(debug_info));
    }
}

impl Drop for Globals {
    fn drop(&mut self) {
        debug!("Globals::drop");
        self.app_sender.send(app::Action::BeforeTerminating);

        // NOTE: なぜかスレッドが停止しない。
        // for j in join_handles {
        //     j.join().unwrap();
        // }
        thread::sleep(Duration::from_secs(3));
        debug!("Globals::drop sleep end");
    }
}

/// グローバル変数を使って処理を行う
///
/// この関数はメインスレッド上でのみ実行できる
fn with_globals<F>(f: F)
where
    F: FnOnce(&mut Globals),
{
    if let Some(mutex) = GLOBALS.get() {
        let mut guard = mutex.lock().unwrap();
        match guard.as_mut() {
            Some(wrapper) => {
                // メインスレッド上でなければエラーになる
                let globals_mut = wrapper.get_mut();

                f(globals_mut)
            }
            None => {
                warn!("with_globals: Destroyed");
            }
        }
    } else {
        warn!("with_globals: Not initialized");
    }
}

/// ロガーを設定する
///
/// 書き込み先のファイルはカレントディレクトリの `hsp3debug.log`
fn init_logger() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let logger = FileLogger::new(&PathBuf::from("hsp3debug.log")).expect("logger");
    log::set_max_level(log_level);
    log::set_boxed_logger(Box::new(logger)).expect("set_logger");
    debug!("ロガーが設定されました (レベル: {:?})", log_level);
}

fn init_globals(globals: Globals) {
    debug!("init_globals (thread={:?})", thread::current().id());
    debug_assert!(GLOBALS.get().is_none());

    GLOBALS
        .set(Mutex::new(Some(SingleThread::new(globals))))
        .unwrap_or_else(|_| unreachable!());
}

fn drop_globals() {
    debug!("drop_globals (thread={:?})", thread::current().id());
    match GLOBALS.get() {
        Some(mutex) => {
            let mut guard = mutex.lock().unwrap();
            guard.take();
        }
        None => {
            debug!("drop_globals: Not initializd");
        }
    }
}

/// すべてのウィンドウにメッセージを送る。
/// NOTE: GUI 版の HSP ランタイムは、何らかのウィンドウメッセージを受け取るまでデバッグモードを「実行」に戻しても実行を再開しない。
unsafe fn tap_all_windows() {
    #[cfg(windows)]
    {
        winapi::um::winuser::PostMessageW(
            winapi::um::winuser::HWND_BROADCAST,
            winapi::um::winuser::WM_NULL,
            0,
            0,
        );
    }
}

/// wait/await などで停止するたびに呼ばれる。
unsafe extern "C" fn my_msgfunc(_hspctx: *mut hspsdk::HSPCTX) {
    with_globals(|g| g.on_msgfunc_called())
}

// -----------------------------------------------
// エクスポートされる関数
// -----------------------------------------------

#[cfg(windows)]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn DllMain(_dll_module: HINSTANCE, reason: DWORD, _: LPVOID) -> BOOL {
    match reason {
        // DLLがロードされたとき (= プログラムの開始時)
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
            debug!("DllMain: Attach");
            init_logger();
        }
        // DLLがアンロードされたとき (= プログラムの終了時)
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            debug!("DllMain: Detach");
            drop_globals();
        }
        _ => {}
    }
    TRUE
}

#[cfg(not(windows))]
#[no_mangle]
pub extern "C" fn hdg_init() {
    init_logger();
}

#[cfg(not(windows))]
#[no_mangle]
pub extern "C" fn hdg_terminate() {
    drop_globals;
}

// -----------------------------------------------
// HSPから呼ばれる関数
// -----------------------------------------------

/// 初期化。HSP ランタイムから最初に呼ばれる。
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debugini(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    _p2: i32,
    _p3: i32,
    _p4: i32,
) -> i32 {
    debug!("debugini");
    init_globals(Globals::new(hsp_debug));
    return 0;
}

/// assert/logmes 命令の実行時に呼ばれる。
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debug_notice(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    reason: i32,
    _p3: i32,
    _p4: i32,
) -> i32 {
    debug!("debug_notice({reason})");

    match reason as isize {
        hspsdk::DEBUG_NOTICE_LOGMES => {
            with_globals(|g| g.on_logmes_called());
            return 0;
        }
        hspsdk::DEBUG_NOTICE_STOP => {
            with_globals(|g| g.on_stopped());
            return 0;
        }
        _ => {
            warn!("debug_notice: Unknown reason");
            return 0;
        }
    }
}
