use logger;
use std;
use std::str;

#[cfg(target_os = "windows")]
use winapi;

/// ゼロ終端の utf-16 文字列に変換する。(Win32 API に渡すのに使う。)
pub(crate) fn to_u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// HSP ランタイムが扱う文字列を utf-8 に変換する。
/// NOTE: utf-8 版ではないので cp932 が来る。いまのところ ascii でない文字は捨てている。
pub(crate) fn string_from_hsp_str<'a>(p: *mut i8) -> String {
    let s = p as *mut u8;

    // ゼロ終端を探して、文字列の長さを調べる。
    // NOTE: バッファオーバーフローを避けるため、適当な長さで探索を打ち切る。
    for i in 0..4096 {
        if unsafe { *s.add(i) } == 0 {
            let bytes = unsafe { std::slice::from_raw_parts_mut(s, i) };
            return String::from_utf8_lossy(bytes).into_owned();
        }
    }

    "[COULD NOT READ]".to_owned()
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
