use logger;
use std;
use std::str;
use winapi;

/// ゼロ終端の utf-16 文字列に変換する。(Win32 API に渡すのに使う。)
pub(crate) fn to_u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// メッセージボックスを表示する。
pub(crate) fn message_box(message: &str) {
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

/// エラーメッセージを出力して異常終了する。(デバッグ用)
pub(crate) fn failwith<T: std::fmt::Debug>(error: T) -> ! {
    let message = format!("ERROR: {:?}", error);
    logger::log(&message);
    message_box(&message);
    panic!()
}
