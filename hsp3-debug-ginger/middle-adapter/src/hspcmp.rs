//! `hspcmp.dll` を使ってコンパイル処理を行う

// 注意:
// 32ビット版のツールチェイン ( i686-pc-windows-msvc) でのみ動作する
//
// Cargo.toml に次が必要:
// encoding = "0.2.33"
// libloading = "0.8"
// winapi = { version = "0", features = ["errhandlingapi", "namedpipeapi", "winbase", "winuser"] }
//
// 実行時、`hspcmp.dll` が必要。
// カレントディレクトリにあるか、パスの通ったディレクトリにあるか、オプションでパスを指定すること
//
// 参考:
//
// - ドキュメント:
//      - hspNN/common/hspcmp.as
//      - hspNN/doclib/hspcmp.txt
// - ソースコード:
//      - openhsp/src/hspcmp/win32dll/hspcmp3.cpp, hspcmp.def
// - その他:
//      - hsp3-ginger/hsp3-debug-window-adapter/vscode-ext/dist/builder.hsp, builder.md

use encoding::{DecoderTrap, Encoding};
use std::{
    error::Error,
    ffi::{c_char, c_void},
    path::{Path, PathBuf},
    ptr::{addr_of_mut, null_mut},
    sync::Mutex,
};

// HSP function types
type Func0 = unsafe extern "stdcall" fn(i32, i32, i32, i32) -> i32;
type Func1 = unsafe extern "stdcall" fn(*mut c_char, i32, i32, i32) -> i32;
type Func5 = unsafe extern "stdcall" fn(*mut c_char, *mut c_char, i32, i32) -> i32;
type Func6 = unsafe extern "stdcall" fn(*mut c_void, *mut c_char, i32, i32) -> i32;
type Func16 = unsafe extern "stdcall" fn(i32, i32, i32, *mut c_char) -> i32;

#[derive(Default)]
pub(crate) struct CompileOptions<'a> {
    /// `hspcmp.dll` へのパス
    pub lib_path: Option<&'a Path>,
    pub refname: Option<&'a str>,

    /// 出力されるオブジェクトファイルの名前
    pub objname: Option<&'a str>,

    /// `common` ディレクトリへのパス
    ///
    /// (既定値: `hsp3_root` の直下のcommon)
    pub compath: Option<&'a Path>,

    /// スクリプトファイルをUTF-8とみなして読み込む
    ///
    /// trueなら、`hsc_comp` の第2引数のフラグ32を立てる
    /// (既定値: false)
    pub utf8_input: bool,

    /// 文字列データをUTF-8コードに変換して出力する
    ///
    /// trueなら、`hsc_comp` の第1引数p1にフラグ4を立てる。
    /// (既定値: false)
    pub utf8_output: bool,
}

#[derive(Default)]
pub(crate) struct CompileOutput {
    pub ok: bool,
    pub runtime: String,
    pub message: String,
}

#[cfg(target_pointer_width = "32")]
/// スクリプトファイルをコンパイルして、オブジェクトファイルを生成する
///
/// パラメータ:
///
/// - `script` (`*.hsp` または `hsptmp` へのパス)
/// - `hsp3_root` HSPのインストールディレクトリ
/// - `options` コンパイルオプション (`hspcmp.txt` を参照)
///
/// 返り値:
///
/// - `Ok`: コンパイルの処理が成功またはコンパイルエラーになったとき。
/// - `Err`: ライブラリのロードなどに失敗したとき
pub(crate) fn compile(
    script: &Path,
    hsp3_root: &Path,
    options: CompileOptions<'_>,
) -> Result<CompileOutput, Box<dyn Error>> {
    let _lock = MUTEX.lock()?;

    let CompileOptions {
        lib_path: lib_path_opt,
        refname: refname_opt,
        objname: objname_opt,
        compath: compath_opt,
        utf8_input,
        utf8_output,
    } = options;

    let lib_path = match lib_path_opt {
        Some(it) => PathBuf::from(it),
        None => PathBuf::from(hsp3_root).join("hspcmp.dll"),
    };

    let objname = match objname_opt {
        Some(it) => it,
        None => "obj",
    };
    let mut objname_c_str = to_c_str(&objname);

    // `hsc_comp` の第1引数
    // (1: デバッグモードでコンパイルする)
    let mut compile_opts = 1;
    // `hsc_comp` の第2引数
    let mut pp_opts = 0;
    // `hsc_comp` の第3引数
    // (1: デバッグウィンドウを表示する)
    let debug_mode = 1;

    if utf8_input {
        // (32: スクリプトファイルをUTF-8とみなして読み込む (utf-8 input))
        pp_opts |= 32;
    }
    if utf8_output {
        // (4: 文字列をUTF-8に変換して生成されるファイルに埋め込む (utf-8 output))
        compile_opts |= 4;
    }

    let mut runtime = String::new();

    // 動的ライブラリをロードする:
    let lib = unsafe { libloading::Library::new(&lib_path)? };

    // 関数シンボルを取り出す:
    let hsc_ini: libloading::Symbol<'_, Func6> = unsafe { lib.get(b"_hsc_ini@16\0")? };
    let hsc_refname: libloading::Symbol<'_, Func6> = unsafe { lib.get(b"_hsc_refname@16\0")? };
    let hsc_objname: libloading::Symbol<'_, Func6> = unsafe { lib.get(b"_hsc_objname@16\0")? };
    let hsc_compath: libloading::Symbol<'_, Func6> = unsafe { lib.get(b"_hsc_compath@16\0")? };
    let hsc_comp: libloading::Symbol<'_, Func0> = unsafe { lib.get(b"_hsc_comp@16\0")? };
    let hsc_getmes: libloading::Symbol<'_, Func1> = unsafe { lib.get(b"_hsc_getmes@16\0")? };
    // let hsc_clrmes: libloading::Symbol<'_, Func0> = unsafe { lib.get(b"_hsc_clrmes@16\0")? };
    // let hsc_ver: libloading::Symbol<'_, Func16> = unsafe { lib.get(b"_hsc_ver@16\0")? };
    // let hsc_bye: libloading::Symbol<'_, Func0> = unsafe { lib.get(b"_hsc_bye@16\0")? };
    // let hsc3_getsym: libloading::Symbol<'_, Func0> = unsafe { lib.get(b"_hsc3_getsym@16\0")? };
    let hsc3_messize: libloading::Symbol<'_, Func1> = unsafe { lib.get(b"_hsc3_messize@16\0")? };
    // let hsc3_make: libloading::Symbol<'_, Func6> = unsafe { lib.get(b"_hsc3_make@16\0")? };
    let hsc3_getruntime: libloading::Symbol<'_, Func5> =
        unsafe { lib.get(b"_hsc3_getruntime@16\0")? };
    // let hsc3_run: libloading::Symbol<'_, Func1> = unsafe{lib.get(b"_hsc3_run@16\0")?};

    {
        let compath = match compath_opt {
            Some(it) => PathBuf::from(it),
            None => PathBuf::from(&hsp3_root).join("common"),
        };
        let mut compath: Vec<u8> = compath.to_str().unwrap().as_bytes().to_owned();
        compath.push(b'\\');
        compath.push(0);

        unsafe { hsc_compath(null_mut(), compath.as_mut_ptr() as *mut i8, 0, 0) };
    }

    {
        let mut fname = to_c_str(script.to_str().unwrap());
        unsafe { hsc_ini(null_mut(), fname.as_mut_ptr() as *mut c_char, 0, 0) };
    }

    if let Some(refname) = refname_opt {
        let mut refname = to_c_str(&refname);
        unsafe { hsc_refname(null_mut(), refname.as_mut_ptr() as *mut c_char, 0, 0) };
    }

    if objname != "obj" {
        unsafe { hsc_objname(null_mut(), objname_c_str.as_mut_ptr() as *mut c_char, 0, 0) };
    }

    let ok = 'compile: {
        let stat = unsafe { hsc_comp(compile_opts, pp_opts, debug_mode, 0) };
        if stat != 0 {
            break 'compile false;
        }

        let mut runtime_name_buf: Vec<u8> = vec![0; 1024];
        let mut objname = to_c_str(&objname);

        unsafe {
            hsc3_getruntime(
                runtime_name_buf.as_mut_ptr() as *mut c_char,
                objname.as_mut_ptr() as *mut c_char,
                0,
                0,
            )
        };
        runtime = from_c_str(&runtime_name_buf);
        if runtime == "" {
            runtime = "hsp3.exe".to_string();
        }
        true
    };

    // コンパイルメッセージを取り出す:
    let mut messize: i32 = 0;
    unsafe { hsc3_messize(addr_of_mut!(messize) as *mut c_char, 0, 0, 0) };
    assert!(messize >= 1);

    let mut mesbuf: Vec<u8> = vec![0; messize as usize];
    unsafe { hsc_getmes(mesbuf.as_mut_ptr() as *mut i8, 0, 0, 0) };

    let mut message = String::new();
    {
        let len = mesbuf.iter().position(|&b| b == 0).unwrap_or(0);
        for line in mesbuf[..len].split(|&b| b == b'\n') {
            // 末尾のCRを取り除く
            let line = line.strip_suffix(b"\r").unwrap_or(line);

            // スクリプトがUTF-8の場合、スクリプトの引用である行はすでにUTF-8なので、変換せずに処理する
            if utf8_input && contains(&line, b"-->") {
                if let Ok(line) = std::str::from_utf8(&line) {
                    if !message.is_empty() {
                        message.push_str("\r\n");
                    }
                    message += line.trim_end();
                    continue;
                }
            }

            if !message.is_empty() {
                message.push_str("\r\n");
            }
            convert_to_utf8(&line, &mut message);
        }
    }

    Ok(CompileOutput {
        ok,
        runtime,
        message,
    })
}

// note: ライブラリのロードやアクセスを含む関数が複数のスレッドから呼ばれると壊れる。ロックをとることで防ぐ
static MUTEX: Mutex<()> = Mutex::new(());

// ===============================================

/// バイト列が特定の部分列を含むか (`instr() >= 0` と同じ)
fn contains(slice: &[u8], pattern: &[u8]) -> bool {
    slice.windows(pattern.len()).any(|sub| sub == pattern)
}

/// Rustの文字列をゼロ終端バイト列(C形式)に変換する
///
/// `str` は常にUTF-8エンコーディングなので、UTF-8エンコーディングのバイト列が返される
fn to_c_str(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.as_bytes().to_owned();
    v.push(0);
    v
}

/// ゼロ終端バイト列(C形式)をRustの文字列に変換する
///
/// 入力データはUTF-8エンコーディングでなければいけない
fn from_c_str(s: &[u8]) -> String {
    let len = s.iter().position(|&b| b == 0).unwrap_or(0);
    String::from_utf8_lossy(&s[..len]).to_string()
}

/// shift_jisのバイト列をUTF-8エンコーディングに変換して、`out` の末尾に連結する
fn convert_to_utf8(sjis: &[u8], out: &mut String) {
    encoding::all::WINDOWS_31J
        .decode_to(&sjis, DecoderTrap::Replace, out)
        .unwrap();
}

// ===============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// コンパイルに成功するケース
    #[test]
    fn it_works() {
        let hsp3_root = PathBuf::from(option_env!("HSP3_ROOT").unwrap_or(r"C:\bin\hsp36"));
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(r"examples\inc_loop.hsp");

        let output = compile(&script, &hsp3_root, CompileOptions::default()).expect("compile");

        assert!(output.ok);
        assert!(output.message.contains("#No error detected."));
        assert_eq!(output.runtime, "hsp3.exe");
    }

    /// コンパイルエラー (スクリプトがshift_jisのとき)
    #[test]
    fn error_message_sjis() {
        let hsp3_root = PathBuf::from(option_env!("HSP3_ROOT").unwrap_or(r"C:\bin\hsp36"));
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(r"tests\hsp\compile_error_sjis.hsp");

        let output = compile(&script, &hsp3_root, CompileOptions::default()).expect("compile");

        // panic!("ok={:?} message={:?}", output.ok, output.message);
        assert!(!output.ok);
        assert!(output.message.contains("compile_error_sjis.hsp(4)"));
        assert!(output.message.contains("パラメーター式の記述が無効です"));
        assert!(output.message.contains("ここで構文エラー"));
    }

    /// コンパイルエラー (スクリプトがUTF-8のとき)
    #[test]
    fn error_message_utf8() {
        let hsp3_root = PathBuf::from(option_env!("HSP3_ROOT").unwrap_or(r"C:\bin\hsp36"));
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(r"tests\hsp\compile_error_utf8.hsp");

        let output = compile(
            &script,
            &hsp3_root,
            CompileOptions {
                utf8_input: true,
                ..CompileOptions::default()
            },
        )
        .expect("compile");

        // panic!("ok={:?} message={:?}", output.ok, output.message);
        assert!(!output.ok);
        assert!(output.message.contains("compile_error_utf8.hsp(6)"));
        assert!(output.message.contains("パラメーター式の記述が無効です"));
        assert!(output.message.contains("ここで構文エラー"));
    }
}
