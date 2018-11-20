use logger;
use std;
use std::{iter, ptr, str};

#[cfg(target_os = "windows")]
use winapi;

#[cfg(windows)]
use std::{ffi, os::windows::ffi::OsStrExt};

/// ゼロ終端の utf-16 文字列に変換する。(Win32 API に渡すのに使う。)
pub(crate) fn to_u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ANSI 文字列 (日本語版 Windows では cp932) を utf-16 に変換する。
#[cfg(windows)]
fn ansi_to_wide_string(s: &[u8]) -> Vec<u16> {
    let size = unsafe {
        winapi::um::stringapiset::MultiByteToWideChar(
            winapi::um::winnls::CP_ACP,
            0,
            s.as_ptr() as *mut i8,
            s.len() as i32,
            ptr::null_mut(),
            0,
        )
    } as usize;

    let buf = vec![0; size];
    unsafe {
        winapi::um::stringapiset::MultiByteToWideChar(
            winapi::um::winnls::CP_ACP,
            0,
            s.as_ptr() as *mut i8,
            s.len() as i32,
            buf.as_ptr() as *mut u16,
            buf.len() as i32,
        )
    };

    buf
}

#[cfg(windows)]
fn ansi_from_wide_string(s: &[u16]) -> Vec<u8> {
    let size = unsafe {
        winapi::um::stringapiset::WideCharToMultiByte(
            winapi::um::winnls::CP_ACP,
            0,
            s.as_ptr() as *mut u16,
            s.len() as i32,
            ptr::null_mut(),
            0,
            ptr::null(),
            ptr::null_mut(),
        )
    } as usize;

    let buf = vec![0; size + 1];
    unsafe {
        winapi::um::stringapiset::WideCharToMultiByte(
            winapi::um::winnls::CP_ACP,
            0,
            s.as_ptr() as *mut u16,
            s.len() as i32,
            buf.as_ptr() as *mut i8,
            buf.len() as i32,
            ptr::null(),
            ptr::null_mut(),
        )
    };

    buf
}

/// HSP ランタイムが扱う文字列を utf-8 に変換する。
pub(crate) fn string_from_hsp_str(p: *mut i8) -> String {
    let s = p as *mut u8;

    // ゼロ終端を探して、文字列の長さを調べる。
    // NOTE: バッファオーバーフローを避けるため、適当な長さで探索を打ち切る。
    for i in 0..4096 {
        if unsafe { *s.add(i) } == 0 {
            let bytes = unsafe { std::slice::from_raw_parts_mut(s, i) };

            #[cfg(windows)]
            {
                let wide = ansi_to_wide_string(bytes);
                return String::from_utf16_lossy(&wide);
            }

            #[cfg(not(windows))]
            {
                return String::from_utf8_lossy(bytes).into_owned();
            }
        }
    }

    "[COULD NOT READ]".to_owned()
}

pub(crate) fn hsp_str_from_string(s: &str) -> Vec<u8> {
    #[cfg(windows)]
    {
        ansi_from_wide_string(&to_u16s(s))
    }
    #[cfg(not(windows))]
    {
        s.bytes().chain(iter::once(0)).collect()
    }
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
