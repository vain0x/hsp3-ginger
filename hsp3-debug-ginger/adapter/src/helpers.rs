use std;
use std::str;

#[cfg(windows)]
use {std::ptr, winapi};

/// ゼロ終端の utf-16 文字列に変換する。(Win32 API に渡すのに使う。)
#[allow(unused)]
pub(crate) fn to_u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ANSI 文字列 (日本語版 Windows では cp932) を utf-16 に変換する。
#[cfg(windows)]
#[allow(unused)]
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
pub(crate) fn string_from_hsp_str(s: *const u8) -> String {
    // ゼロ終端を探して、文字列の長さを調べる。
    // NOTE: バッファオーバーフローを避けるため、適当な長さで探索を打ち切る。
    for i in 0..4096 {
        if unsafe { *s.add(i) } == 0 {
            let bytes = unsafe { std::slice::from_raw_parts(s, i) };
            return string_from_hsp_str_slice(bytes);
        }
    }

    "[COULD NOT READ]".to_owned()
}

fn string_from_hsp_str_slice(bytes: &[u8]) -> String {
    #[cfg(windows)]
    {
        return cp932_to_utf8(bytes);
    }

    #[cfg(not(windows))]
    {
        return String::from_utf8_lossy(bytes).into_owned();
    }
}

#[allow(unused)]
pub(crate) fn hsp_str_from_string(s: &str) -> Vec<u8> {
    #[cfg(windows)]
    {
        ansi_from_wide_string(&to_u16s(s))
    }
    #[cfg(not(windows))]
    {
        s.bytes().chain(std::iter::once(0)).collect()
    }
}

/// メッセージボックスを表示する。
#[allow(unused)]
pub(crate) fn message_box(message: &str) {
    #[cfg(windows)]
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

/// utf-8 版ではない HSP の文字列 (cp932 エンコード) を utf-8 の文字列に変換する。
/// 変換できない部分は `?` になる。
#[allow(unused)]
fn cp932_to_utf8(mut s: &[u8]) -> String {
    let cp932 = encoding::label::encoding_from_windows_code_page(932).unwrap();

    while let Some(0) = s.last() {
        s = &s[0..s.len() - 1];
    }
    cp932.decode(s, encoding::DecoderTrap::Replace).unwrap()
}

/// utf-8 の文字列を utf-8 版ではない HSP の文字列 (cp932 エンコード) に変換する。
#[allow(unused)]
fn utf8_to_cp932(s: &str) -> Vec<u8> {
    let cp932 = encoding::label::encoding_from_windows_code_page(932).unwrap();
    let mut buf = cp932.encode(s, encoding::EncoderTrap::Replace).unwrap();
    buf.push(0);
    buf
}

mod tests {
    #[test]
    fn test_encoding_cp932() {
        let hello = vec![
            130, 177, 130, 241, 130, 201, 130, 191, 130, 205, 144, 162, 138, 69, 0,
        ];
        assert_eq!(super::utf8_to_cp932("こんにちは世界"), hello);
        assert_eq!(super::cp932_to_utf8(&hello), "こんにちは世界");

        assert_eq!(super::cp932_to_utf8(&super::utf8_to_cp932("✔")), "?");
    }
}
