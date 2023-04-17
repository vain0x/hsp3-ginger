use encoding::{DecoderTrap, Encoding};
use std::{fs, path::Path, str};

/// テキストファイルを shift_jis または UTF-8 として読む。
pub(crate) fn read_file(file_path: &Path, out: &mut String) -> bool {
    // バイナリで読む。
    let contents = match fs::read(file_path).ok() {
        None => return false,
        Some(it) => it,
    };

    // 可能ならUTF-8として読む。
    match str::from_utf8(&contents) {
        Ok(text) => {
            *out += text;
            return true;
        }
        Err(_) => {}
    }

    // shift_jisから変換する。
    encoding::all::WINDOWS_31J
        .decode_to(&contents, DecoderTrap::Strict, out)
        .is_ok()
}

/// テキストファイルを可能ならshift_jisとして読み、ダメだったらUTF-8として読む。
pub(crate) fn read_sjis_file(file_path: &Path, out: &mut String) -> bool {
    debug_assert_eq!(out.len(), 0);

    let contents = match fs::read(file_path).ok() {
        None => return false,
        Some(it) => it,
    };

    let result = encoding::all::WINDOWS_31J.decode_to(&contents, DecoderTrap::Strict, out);
    if result.is_ok() {
        return true;
    }
    out.clear();

    match str::from_utf8(&contents) {
        Ok(text) => {
            *out += text;
            true
        }
        Err(_) => false,
    }
}
