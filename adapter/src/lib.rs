extern crate env_logger;
extern crate libc;
extern crate ws;

#[cfg(target_os = "windows")]
extern crate winapi;

#[macro_use]
extern crate log;

mod connection;
mod helpers;
mod hsprt;
mod hspsdk;
mod logger;

use std::cell::UnsafeCell;
use std::sync::mpsc;
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::*;

/// debug_notice 通知の原因を表す。 (p2 の値。)
const DEBUG_NOTICE_STOP: isize = 0;
const DEBUG_NOTICE_LOGMES: isize = 1;

static mut HSP_DEBUG: Option<UnsafeCell<Option<*mut hspsdk::HSP3DEBUG>>> = None;

static mut HSP_MSGFUNC: Option<unsafe extern "C" fn(*mut hspsdk::HSPCTX)> = None;

static mut HSP_SENDER: Option<mpsc::Sender<HspAction>> = None;

static mut HSP_RECEIVER: Option<mpsc::Receiver<HspAction>> = None;

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
            // HSP が wait/await で中断しているときでなければ無視されるので、次に停止したときにモードの変更を行うように予約する。
            send_action(HspAction::SetMode(mode));
        }
    }
}

unsafe fn init_mod() {
    let (sender, receiver) = mpsc::channel();
    HSP_SENDER = Some(sender);
    HSP_RECEIVER = Some(receiver);
}

/// クレートの static 変数を初期化などを行なう。
/// ここでエラーが起こるとめんどうなので、Mutex や RefCell などを初期化するにとどめて、複雑なオブジェクトの生成は遅延しておく。
fn init_crate() {
    logger::init_mod();
    connection::init_mod();
    unsafe { init_mod() };
}

unsafe fn set_hsp_debug(hsp_debug: *mut hspsdk::HSP3DEBUG) {
    HSP_DEBUG = Some(UnsafeCell::new(Some(hsp_debug)));
}

unsafe fn hook_msgfunc(hspctx: *mut hspsdk::HSP3DEBUG) {
    let hspctx: &mut hspsdk::HSPCTX = &mut *(*hspctx).hspctx;
    HSP_MSGFUNC = hspctx.msgfunc;

    hspctx.msgfunc = Some(msgfunc);
}

fn with_hsp_debug<R, F>(f: F) -> R
where
    F: FnOnce(&mut hspsdk::HSP3DEBUG) -> R,
{
    unsafe {
        let cell: &mut UnsafeCell<_> = HSP_DEBUG.as_mut().unwrap();
        let dp: *mut hspsdk::HSP3DEBUG = (*cell.get()).unwrap();
        let d = &mut *dp;
        f(d)
    }
}

fn send_action(action: HspAction) {
    unsafe {
        let sender = HSP_SENDER.as_ref().unwrap();
        sender.send(action);
    }
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
    });
}

/// wait/await などで停止するたびに呼ばれる。
unsafe extern "C" fn msgfunc(hspctx: *mut hspsdk::HSPCTX) {
    {
        let hspctx = &mut *hspctx;
        let receiver = HSP_RECEIVER.as_mut().unwrap();
        while let Ok(action) = receiver.try_recv() {
            do_action(hspctx, action);
        }

        let stat = &mut hspctx.stat;
        *stat = hspctx.runmode as i32;
    }

    let msgfunc = HSP_MSGFUNC.unwrap();
    msgfunc(hspctx);
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

    unsafe { set_hsp_debug(hsp_debug) };

    unsafe { hook_msgfunc(hsp_debug) };

    connection::Connection::spawn(HspDebugImpl);
    return p2 * 10000 + p3 * 100 + p4;
}

#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debug_notice(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    cause: i32,
    p3: i32,
    p4: i32,
) -> i32 {
    let hspctx: &mut hspsdk::HSPCTX = unsafe { &mut *(*hsp_debug).hspctx };

    match cause as isize {
        DEBUG_NOTICE_LOGMES => {
            // NOTE: utf8 版ではないので cp932
            let given = hspctx.stmp as *mut u8;
            let bytes = helpers::multibyte_str_from_pointer(given);
            let message = String::from_utf8_lossy(bytes);
            logger::log(&message);
            return 0;
        }
        DEBUG_NOTICE_STOP => {}
        _ => {
            logger::log("debug_notice with unknown cause");
            return 0;
        }
    }

    static mut COUNTER: i32 = 0;

    unsafe {
        let c = COUNTER;
        COUNTER += 1;

        let hspctx: &mut hspsdk::HSPCTX = &mut *(*hsp_debug).hspctx;
        let stat = &mut hspctx.stat;
        // *stat = c;
    }

    return 0;
}
