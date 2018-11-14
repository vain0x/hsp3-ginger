use logger;
use std;
use std::str;

#[cfg(target_os = "windows")]
use winapi;

/// ゼロ終端の utf-16 文字列に変換する。(Win32 API に渡すのに使う。)
pub(crate) fn to_u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// マルチバイト文字列を指すポインタを、ゼロ終端を探すことでスライスにする。
pub(crate) fn multibyte_str_from_pointer(s: *mut u8) -> &'static mut [u8] {
    // NOTE: 適当な長さで探索を打ち切る。この範囲にゼロがなければ、バッファオーバーフローを起こす可能性がある。
    for i in 0..4096 {
        if unsafe { *s.add(i) } == 0 {
            return unsafe { std::slice::from_raw_parts_mut(s, i) };
        }
    }
    panic!()
}

/// メッセージボックスを表示する。
pub(crate) fn message_box(message: &str) {
    #[cfg(target_os = "windows")]
    {
        let message = to_u16s(&message);
        let caption = to_u16s("hsp3-debug-ginger-adapter");

        unsafe {
            winapi::um::winuser::MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                caption.as_ptr(),
                winapi::um::winuser::MB_OK,
            );
        }
    }
}

/// エラーメッセージを出力して異常終了する。(デバッグ用)
pub(crate) fn failwith<T: std::fmt::Debug>(error: T) -> ! {
    #[cfg(target_os = "windows")]
    {
        let message = format!("ERROR: {:?}", error);
        logger::log(&message);
        message_box(&message);
        panic!()
    }
    #[cfg(not(target_os = "windows"))]
    {
        panic!("ERROR: {:?}", error)
    }
}
