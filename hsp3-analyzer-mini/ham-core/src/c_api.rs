//! C言語や HSP3 などから利用するための関数群

use super::*;
use crate::analyzer::Analyzer;
use lsp_types::{HoverContents, MarkedString, Position, Url};
use std::{os::raw::c_char, ptr::null_mut, slice, str};

const TRUE: i32 = 1;
const FALSE: i32 = 0;

pub struct HamInstance {
    analyzer: Analyzer,
}

unsafe fn str_from_raw_parts(data: *const c_char, len: i32) -> Option<&'static str> {
    if data.is_null() {
        error!("pointer must be non-null {:?}", data);
        return None;
    }

    if len < 0 {
        error!("length must be non-negative {:?}", len);
        return None;
    }

    match str::from_utf8(slice::from_raw_parts(data as *const u8, len as usize)) {
        Ok(x) => Some(x),
        Err(e) => {
            error!("string must be utf-8 {:?}", e);
            None
        }
    }
}

unsafe fn str_assign(dest: *mut c_char, dest_len: *mut i32, src: &str) {
    // バッファサイズは超えないようにする。
    let mut len = ((*dest_len).max(0) as usize).min(src.len());

    // 文字の途中でぶち切ってしまうときは、中途半端な部分を捨てる。
    while len >= 1 && !src.is_char_boundary(len) {
        len -= 1;
    }

    slice::from_raw_parts_mut(dest as *mut u8, len).copy_from_slice(&src.as_bytes()[..len]);
    *dest_len = len as i32;
}

unsafe fn url_from_raw_parts(uri: *const c_char, uri_len: i32) -> Option<Url> {
    let uri = str_from_raw_parts(uri, uri_len)?;
    match Url::parse(uri) {
        Ok(uri) => Some(uri),
        Err(err) => {
            error!("expected a valid URI {:?}", err);
            None
        }
    }
}

fn position_from_raw(line: i32, character: i32) -> Option<Position> {
    if line < 0 {
        error!("line can't be negative {:?}", line);
        return None;
    }

    if character < 0 {
        error!("character can't be negative {:?}", character);
        return None;
    }

    // FIXME: 列番号をエンコーディングに基づいて変換する？
    Some(Position::new(line as u32, character as u32))
}

fn marked_string_to_string(it: MarkedString) -> String {
    match it {
        MarkedString::String(text) => text,
        MarkedString::LanguageString(s) => s.value,
    }
}

#[no_mangle]
pub extern "C" fn ham_init() {
    // FIXME: ログレベルなどを設定可能にする。(logmes に吐きたい。)
    crate::lsp_server::lsp_log::init_log();
}

// FIXME: オプションを設定できるようにする。
#[no_mangle]
pub unsafe extern "C" fn ham_create(
    hsp3_root: *const c_char,
    hsp3_root_len: i32,
) -> *mut HamInstance {
    let hsp3_root = match str_from_raw_parts(hsp3_root, hsp3_root_len) {
        Some(x) => PathBuf::from(x),
        None => return null_mut(),
    };

    let mut instance = HamInstance {
        analyzer: Analyzer::new(hsp3_root),
    };

    instance.analyzer.did_initialize();

    // Rust の所有権ルールから外して、ネイティブポインタに変換する。ham_destroy で破棄してもらう。
    Box::into_raw(Box::new(instance))
}

#[no_mangle]
pub unsafe extern "C" fn ham_destroy(instance: *mut HamInstance) -> i32 {
    if instance.is_null() {
        return FALSE;
    }

    let mut instance = Box::from_raw(instance);
    instance.analyzer.shutdown();

    drop(instance);
    TRUE
}

#[no_mangle]
pub unsafe extern "C" fn ham_doc_did_open(
    instance: *mut HamInstance,
    uri: *const c_char,
    uri_len: i32,
    version: i32,
    text: *const c_char,
    text_len: i32,
) -> i32 {
    if instance.is_null() {
        return FALSE;
    }

    let uri = match url_from_raw_parts(uri, uri_len) {
        Some(uri) => uri,
        None => return FALSE,
    };

    let text = match str_from_raw_parts(text, text_len) {
        Some(x) => x.to_string(),
        None => return FALSE,
    };

    (*instance).analyzer.open_doc(uri, version, text);
    TRUE
}

#[no_mangle]
pub unsafe extern "C" fn ham_doc_did_change(
    instance: *mut HamInstance,
    uri: *const c_char,
    uri_len: i32,
    version: i32,
    text: *const c_char,
    text_len: i32,
) -> i32 {
    if instance.is_null() {
        return FALSE;
    }

    let uri = match url_from_raw_parts(uri, uri_len) {
        Some(uri) => uri,
        None => return FALSE,
    };

    let text = match str_from_raw_parts(text, text_len) {
        Some(x) => x.to_string(),
        None => return FALSE,
    };

    (*instance).analyzer.change_doc(uri, version, text);
    TRUE
}

#[no_mangle]
pub unsafe extern "C" fn ham_doc_did_close(
    instance: *mut HamInstance,
    uri: *const c_char,
    uri_len: i32,
) -> i32 {
    if instance.is_null() {
        return FALSE;
    }

    let uri = match url_from_raw_parts(uri, uri_len) {
        Some(uri) => uri,
        None => return FALSE,
    };

    (*instance).analyzer.close_doc(uri);
    TRUE
}

#[no_mangle]
pub unsafe extern "C" fn ham_hover(
    instance: *mut HamInstance,
    uri: *const c_char,
    uri_len: i32,
    position_line: i32,
    position_character: i32,
    output: *mut c_char,
    output_len: *mut i32,
) -> i32 {
    if instance.is_null() {
        return FALSE;
    }

    let uri = match url_from_raw_parts(uri, uri_len) {
        Some(uri) => uri,
        None => return FALSE,
    };

    let position = match position_from_raw(position_line, position_character) {
        Some(position) => position,
        None => return FALSE,
    };

    let contents = match (*instance).analyzer.compute_ref().hover(uri, position) {
        Some(hover) => match hover.contents {
            HoverContents::Scalar(scalar) => marked_string_to_string(scalar),
            HoverContents::Array(contents) => contents
                .into_iter()
                .map(marked_string_to_string)
                .collect::<Vec<_>>()
                .join("\r\n\r\n"),
            HoverContents::Markup(markup) => markup.value,
        },
        None => "".to_string(),
    };

    str_assign(output, output_len, &contents);
    TRUE
}
