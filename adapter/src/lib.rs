//! デバッガーのエントリーポイント。

extern crate env_logger;
extern crate libc;
extern crate serde;
extern crate serde_json;
extern crate ws;

#[cfg(windows)]
extern crate winapi;

#[macro_use]
extern crate serde_derive;

#[allow(unused_imports)]
#[macro_use]
extern crate log;

mod app;
mod connection;
mod debug_adapter_connection;
mod debug_adapter_protocol;
mod helpers;
mod hsprt;
mod hspsdk;
mod logger;

use debug_adapter_protocol as dap;
use hsprt::*;
use std::sync::mpsc;
use std::{cell, ptr, thread, time};

#[cfg(windows)]
use winapi::shared::minwindef::*;

static mut GLOBALS: Option<cell::UnsafeCell<Globals>> = None;

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
    fn create(hsp_debug: *mut hspsdk::HSP3DEBUG) -> Self {
        logger::log("debugini");

        // msgfunc に操作を送信するチャネルを生成する。
        let (sender, hsprt_receiver) = mpsc::channel();
        let (notice_sender, notice_receiver) = mpsc::channel();

        let hsprt_sender = Sender::new(sender, notice_sender);

        let j1 = thread::spawn(move || {
            // HSP ランタイムが停止している状態で処理依頼が来るたびに notice_receiver が通知を受け取り、
            // そのたびにループが進行する。msgfunc に代わって処理を行う。
            // FIXME: ワーカースレッドで globals に触るのはとても危険なので同期化機構を使うべき。
            for _ in notice_receiver {
                with_globals(|g| {
                    g.receive_actions();
                });
            }
            logger::log("[notice] 終了");
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

    fn on_msgfunc_called(&self) {
        self.receive_actions();

        if let Some(default_msgfunc) = self.default_msgfunc {
            let hspctx = self.hspctx() as *mut hspsdk::HSPCTX;
            unsafe { default_msgfunc(hspctx) };
        }
    }

    /// メインスレッド上で実行すべき操作があれば、すべて実行する。なければ何もしない。
    fn receive_actions(&self) {
        while let Ok(action) = self.hsprt_receiver.try_recv() {
            self.do_action(action);
        }
    }

    /// `Action` で指定された操作を実行する。
    fn do_action(&self, action: Action) {
        match action {
            Action::SetMode(mode) => {
                self.do_set_mode(mode);
            }
            Action::GetGlobals { seq } => {
                self.do_get_globals(seq);
            }
        }
    }

    // NOTE: 中断モードへの変更は HSP 側が wait/await で中断しているときに行わなければ無視されるので注意。
    fn do_set_mode(&self, mode: hspsdk::DebugMode) {
        let set = self.hsp_debug().dbg_set.unwrap();
        unsafe { set(mode) };
        unsafe { tap_all_windows() };
    }

    fn do_get_globals(&self, seq: i64) {
        let d = self.hsp_debug();

        let get_varinf = d.get_varinf.unwrap();
        let dbg_close = d.dbg_close.unwrap();

        let p = unsafe { get_varinf(ptr::null_mut(), 0xFF) };
        let var_names = helpers::string_from_hsp_str(p as *const u8);
        unsafe { dbg_close(p) };

        let var_names = var_names.trim_right().split("\n").map(|s| s.trim_right());
        let variables = var_names
            .map(|name| {
                let n = helpers::hsp_str_from_string(name);
                let p = unsafe { get_varinf(n.as_ptr() as *mut i8, 0) };
                // 最初の7行はヘッダーなので無視する。文字列などは複数行になることもあるが、最初の1行だけ取る。
                let value = helpers::string_from_hsp_str(p as *const u8)
                    .lines()
                    .skip(7)
                    .next()
                    .unwrap_or("???")
                    .to_owned();
                unsafe { dbg_close(p) };

                dap::Variable {
                    name: name.to_owned(),
                    value,
                    ty: None,
                    variables_reference: 0,
                }
            }).collect::<Vec<_>>();

        let event = app::DebugResponse::Globals { seq, variables };
        self.app_sender.send(app::Action::DebugEvent(event));
    }

    fn on_logmes_called(&mut self) {
        let message = helpers::string_from_hsp_str(self.hspctx().stmp as *const u8);
        logger::log(&message);
    }

    /// assert などで停止したときに呼ばれる。
    fn on_stopped(&mut self) {
        let d = self.hsp_debug();

        let (file_name, line) = {
            // 実行位置 (ファイル名と行番号) の情報を更新する。
            let curinf = d.dbg_curinf.unwrap();
            unsafe { curinf() };

            let file_name = helpers::string_from_hsp_str(d.fname as *const u8);
            logger::log(&format!("file_name = {:?}", file_name));

            (file_name, d.line)
        };

        // 停止イベントを VSCode 側に通知する。
        self.app_sender
            .send(app::Action::AfterStopped(file_name, line));
    }

    fn terminate(self) {
        let join_handles = {
            let (app_sender, join_handles) = (self.app_sender, self.join_handles);
            app_sender.send(app::Action::BeforeTerminating);
            join_handles
        };

        // NOTE: なぜかスレッドが停止しない。
        // for j in join_handles {
        //     j.join().unwrap();
        // }
        thread::sleep(time::Duration::from_secs(3));
    }
}

/// グローバル変数を使って処理を行う。
fn with_globals<F>(f: F)
where
    F: FnOnce(&mut Globals),
{
    if let Some(cell) = unsafe { GLOBALS.as_ref() } {
        let globals = unsafe { &mut *cell.get() };
        f(globals)
    }
}

/// クレートの static 変数の初期化などを行なう。
/// ここでエラーが起こるとめんどうなので、Mutex などのオブジェクトの生成にとどめる。
fn initialize_crate() {
    logger::initialize_mod();
}

fn deinitialize_crate() {
    logger::log("デバッガーがデタッチされました");

    // 処理を停止させて、グローバル変数をすべてドロップする。
    if let Some(globals_cell) = unsafe { GLOBALS.take() } {
        globals_cell.into_inner().terminate();
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

#[cfg(windows)]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn DllMain(
    _dll_module: HINSTANCE,
    call_reason: DWORD,
    _reserved: LPVOID,
) -> BOOL {
    match call_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {
            initialize_crate();
        }
        winapi::um::winnt::DLL_PROCESS_DETACH => {
            deinitialize_crate();
        }
        _ => {}
    }
    TRUE
}

/// 初期化。HSP ランタイムから最初に呼ばれる。
#[cfg(windows)]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debugini(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    _p2: i32,
    _p3: i32,
    _p4: i32,
) -> i32 {
    let globals = Globals::create(hsp_debug);
    unsafe { GLOBALS = Some(cell::UnsafeCell::new(globals)) };
    return 0;
}

/// assert/logmes 命令の実行時に呼ばれる。
#[cfg(windows)]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debug_notice(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    cause: i32,
    _p3: i32,
    _p4: i32,
) -> i32 {
    match cause as isize {
        hspsdk::DEBUG_NOTICE_LOGMES => {
            with_globals(|g| g.on_logmes_called());
            return 0;
        }
        hspsdk::DEBUG_NOTICE_STOP => {
            with_globals(|g| g.on_stopped());
            return 0;
        }
        _ => {
            logger::log("debug_notice with unknown cause");
            return 0;
        }
    }
}
