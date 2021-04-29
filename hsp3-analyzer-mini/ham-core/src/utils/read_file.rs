use std::{fs, path::Path};

use encoding::{codec::utf_8::UTF8Encoding, DecoderTrap, Encoding, StringWriter};

/// ファイルを shift_jis または UTF-8 として読む。
pub(crate) fn read_file(file_path: &Path, out: &mut impl StringWriter) -> bool {
    // utf-8?
    let content = match fs::read(file_path).ok() {
        None => return false,
        Some(x) => x,
    };

    // shift-jis?
    encoding::all::WINDOWS_31J
        .decode_to(&content, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&content, DecoderTrap::Strict, out))
        .is_ok()
}
