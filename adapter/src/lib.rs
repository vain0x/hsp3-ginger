extern crate libc;
extern crate winapi;
extern crate ws;

mod connection;
mod helpers;
mod hspsdk;
mod logger;

use winapi::shared::minwindef::*;

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

    // message_box("init".to_owned());
    connection::Connection::spawn();
    return p2 * 10000 + p3 * 100 + p4;
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn debug_notice(
    hsp_debug: *mut hspsdk::HSP3DEBUG,
    p2: i32,
    p3: i32,
    p4: i32,
) -> i32 {
    static mut COUNTER: i32 = 0;

    unsafe {
        let c = COUNTER;
        COUNTER += 1;

        let set_run_mode = (*hsp_debug).dbg_set.unwrap();
        set_run_mode(hspsdk::RUNMODE_RUN as i32);

        let s = std::ffi::CString::new(c.to_string()).unwrap();
        let refstr = (*(*hsp_debug).hspctx).refstr;
        std::ptr::copy_nonoverlapping(s.as_ptr(), refstr, hspsdk::HSPCTX_REFSTR_MAX as usize);
    }

    return 0;
}
