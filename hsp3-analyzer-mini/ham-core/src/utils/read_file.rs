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
