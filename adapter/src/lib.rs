extern crate libc;
extern crate winapi;
extern crate ws;

mod connection;
mod helpers;
mod hspsdk;
mod logger;

use winapi::shared::minwindef::*;

/// debug_notice 通知の原因を表す。 (p2 の値。)
const DEBUG_NOTICE_STOP: isize = 0;
const DEBUG_NOTICE_LOGMES: isize = 1;

/// マルチバイト文字列を指すポインタを、ゼロ終端を探すことでスライスにする。
fn multibyte_str_from_pointer(s: *mut u8) -> &'static mut [u8] {
    // NOTE: 適当な長さで探索を打ち切る。この範囲にゼロがなければ、バッファオーバーフローを起こす可能性がある。
    for i in 0..4096 {
        if unsafe { *s.add(i) } == 0 {
            return unsafe { std::slice::from_raw_parts_mut(s, i) };
        }
    }
    panic!()
}

/// クレートの static 変数を初期化などを行なう。
/// ここでエラーが起こるとめんどうなので、Mutex や RefCell などを初期化するにとどめて、複雑なオブジェクトの生成は遅延しておく。
fn init_crate() {
    logger::init_mod();
    connection::init_mod();
}

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
    connection::Connection::spawn();
    return p2 * 10000 + p3 * 100 + p4;
}

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
            let bytes = multibyte_str_from_pointer(given);
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

        let set_run_mode = (*hsp_debug).dbg_set.unwrap();
        set_run_mode(hspsdk::RUNMODE_RUN as i32);

        let hspctx: &mut hspsdk::HSPCTX = &mut *(*hsp_debug).hspctx;
        let stat = &mut hspctx.stat;
        *stat = c;

        // let s = std::ffi::CString::new(c.to_string()).unwrap();
        // let refstr = (*(*hsp_debug).hspctx).refstr;
        // std::ptr::copy_nonoverlapping(s.as_ptr(), refstr, hspsdk::HSPCTX_REFSTR_MAX as usize);
    }

    return 0;
}
