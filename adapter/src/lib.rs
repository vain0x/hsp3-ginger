extern crate env_logger;
extern crate libc;
extern crate ws;

#[cfg(target_os = "windows")]
extern crate winapi;

#[macro_use]
extern crate log;

mod app;
mod connection;
mod helpers;
mod hsprt;
mod hspsdk;
mod logger;

use std::sync::mpsc;
use std::{cell, thread};

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::*;

type HspMsgFunc = Option<unsafe extern "C" fn(*mut hspsdk::HSPCTX)>;

struct Globals {
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    default_msgfunc: HspMsgFunc,
    hsp_sender: mpsc::Sender<HspAction>,
    hsp_receiver: mpsc::Receiver<HspAction>,
    app_sender: app::Sender,
    join_handle: thread::JoinHandle<()>,
}

static mut GLOBALS: Option<cell::UnsafeCell<Globals>> = None;

fn with_globals<R, F>(f: F) -> R
where
    F: FnOnce(&mut Globals) -> R,
{
    let cell = unsafe { GLOBALS.as_ref().unwrap() };
    let globals = unsafe { &mut *cell.get() };
    f(globals)
}

fn with_hsp_debug<R, F>(f: F) -> R
where
    F: FnOnce(&mut hspsdk::HSP3DEBUG) -> R,
{
    with_globals(|g| {
        let hsp_debug = unsafe { &mut *g.hsp_debug };
        f(hsp_debug)
    })
}

/// HSP のメインスレッドで実行すべき操作を表す。
#[derive(Clone, Debug)]
enum HspAction {
    SetMode(hspsdk::DebugMode),
}

#[derive(Clone, Copy, Debug)]
struct HspDebugImpl;

impl hsprt::HspDebug for HspDebugImpl {
    fn set_mode(&mut self, mode: hspsdk::DebugMode) {
        if mode != hspsdk::HSPDEBUG_STOP {
            do_set_mode(mode);
        } else {
            // 中断モードへの変更は、HSP 側が wait/await で中断しているときに行わなければ無視されるので、
            // 次に停止したときに中断モードに変更するよう予約する。
            send_action(HspAction::SetMode(mode));
        }
    }
}

/// HSP ランタイムが次に wait/await を行ったときに処理が実行されるようにする。
fn send_action(action: HspAction) {
    with_globals(|g| {
        g.hsp_sender
            .send(action)
            .map_err(|e| logger::log_error(&e))
            .ok();
    })
}

fn do_action(hspctx: &mut hspsdk::HSPCTX, action: HspAction) {
    match action {
        HspAction::SetMode(mode) => {
            do_set_mode(mode);
        }
    }
}

fn do_set_mode(mode: hspsdk::DebugMode) {
    with_hsp_debug(|d| {
        let set = d.dbg_set.unwrap();
        unsafe { set(mode) };
        unsafe { tap_all_windows() };
    });
}

/// HSP のメッセージ関数をフックする。
/// つまり、 wait/await などの際に `msgfunc` が呼ばれるようにする。
/// 結果として返されるもとのメッセージ関数を `msgfunc` の中で呼び出す必要がある。
unsafe fn hook_msgfunc(hspctx: *mut hspsdk::HSP3DEBUG) -> HspMsgFunc {
    let hspctx: &mut hspsdk::HSPCTX = &mut *(*hspctx).hspctx;
    let default_msgfunc = hspctx.msgfunc;

    hspctx.msgfunc = Some(msgfunc);

    default_msgfunc
}

/// クレートの static 変数の初期化などを行なう。
/// ここでエラーが起こるとめんどうなので、Mutex や RefCell などを初期化するにとどめて、複雑なオブジェクトの生成は遅延しておく。
fn init_crate() {
    logger::init_mod();
}

/// wait/await などで停止するたびに呼ばれる。
unsafe extern "C" fn msgfunc(hspctx: *mut hspsdk::HSPCTX) {
    let hspctx = &mut *hspctx;

    with_globals(|g| {
        // メインスレッド上で実行すべき操作があれば、すべて実行する。なければ何もしない。
        while let Ok(action) = g.hsp_receiver.try_recv() {
            do_action(hspctx, action);
        }

        let default_msgfunc = g.default_msgfunc.unwrap();
        default_msgfunc(hspctx);
    });
}

/// すべてのウィンドウにメッセージを送る。
/// NOTE: GUI 版の HSP ランタイムは、何らかのウィンドウメッセージを受け取るまでデバッグモードを「実行」に戻しても実行を再開しない。
unsafe fn tap_all_windows() {
    #[cfg(target_os = "windows")]
    {
        winapi::um::winuser::PostMessageW(
            winapi::um::winuser::HWND_BROADCAST,
            winapi::um::winuser::WM_NULL,
            0,
            0,
        );
    }
}

#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: DWORD,
    reserved: LPVOID,
) -> BOOL {
    match call_reason {
        winapi::um::winnt::DLL_PROCESS_ATTACH => {}
        winapi::um::winnt::DLL_PROCESS_DETACH => {}
        _ => {}
    }
    TRUE
}

/// 初期化。HSP ランタイムから最初に呼ばれる。
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debugini(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    p2: i32,
    p3: i32,
    p4: i32,
) -> i32 {
    init_crate();

    logger::log("debugini");

    // msgfunc に操作を送信するチャネル。
    let (hsp_sender, hsp_receiver) = mpsc::channel();

    // ワーカースレッドを起動する。
    let app_worker = app::Worker::new(HspDebugImpl);
    let app_sender = app_worker.sender();
    let join_handle = thread::spawn(move || app_worker.run());

    let default_msgfunc = unsafe { hook_msgfunc(hsp_debug) };

    let globals = Globals {
        hsp_debug,
        default_msgfunc,
        hsp_sender,
        hsp_receiver,
        app_sender,
        join_handle,
    };

    unsafe { GLOBALS = Some(cell::UnsafeCell::new(globals)) };

    return p2 * 10000 + p3 * 100 + p4;
}

/// assert/logmes 命令の実行時に呼ばれる。
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debug_notice(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    cause: i32,
    p3: i32,
    p4: i32,
) -> i32 {
    logger::log("debug notice");

    let d: &mut hspsdk::HSP3DEBUG = unsafe { &mut *hsp_debug };
    let hspctx: &mut hspsdk::HSPCTX = unsafe { &mut *d.hspctx };

    match cause as isize {
        hspsdk::DEBUG_NOTICE_LOGMES => {
            // NOTE: utf8 版ではないので cp932
            let given = hspctx.stmp as *mut u8;
            let bytes = helpers::multibyte_str_from_pointer(given);
            let message = String::from_utf8_lossy(bytes);
            logger::log(&message);
            return 0;
        }
        hspsdk::DEBUG_NOTICE_STOP => {}
        _ => {
            logger::log("debug_notice with unknown cause");
            return 0;
        }
    }

    let line = {
        // 実行位置 (ファイル名と行番号) の情報を更新する。
        let curinf = d.dbg_curinf.unwrap();
        unsafe { curinf() };

        d.line
    };

    // 停止イベントを VSCode 側に通知する。
    with_globals(|g| {
        g.app_sender.send(app::Action::EventStop(line));
    });

    return 0;
}
