#[macro_use]
extern crate log;

pub(crate) mod file_changes;
pub(crate) mod file_watcher;
pub(crate) mod lsp_features;
pub(crate) mod lsp_model;
pub(crate) mod lsp_server;
pub(crate) mod text_encoding;

use std::env::ArgsOs;
use std::path::PathBuf;

fn get_help() -> String {
    format!(
        r#"{name} {version}

    USAGE:
        {name} [OPTIONS] [SUBCOMMAND]

    EXAMPLE:
        {name} --hsp3 "C:/hsp3"

    OPTIONS:
        -h, --help      Print help
        -V, --version   Print version
            --hsp3      HSP3 インストールディレクトリ"#,
        name = "hsp3-forgery-lsp",
        version = get_version()
    )
}

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn exit_with_help() -> ! {
    eprintln!("{}", get_help());
    std::process::exit(1)
}

fn exit_with_version() -> ! {
    eprintln!("{}", get_version());
    std::process::exit(1)
}

fn switch_on_args(mut args: ArgsOs) -> Result<(), String> {
    // 最初の引数は自身のパスなので無視する。
    args.next();

    let mut hsp_root = None;
    let mut help = false;
    let mut version = false;

    while let Some(arg) = args.next() {
        match arg.into_string().unwrap_or("".to_string()).as_str() {
            "-h" | "--help" | "help" => {
                help = true;
                break;
            }
            "-V" | "--version" | "version" => {
                version = true;
                break;
            }
            "--hsp3" => match args.next() {
                None => return Err("--hsp3 の引数がありません。".to_string()),
                Some(arg) => {
                    hsp_root = Some(arg);
                }
            },
            arg => return Err(format!("不明な引数: {:?}", arg)),
        }
    }

    if help {
        exit_with_help()
    } else if version {
        exit_with_version()
    } else {
        let hsp_root = PathBuf::from(hsp_root.expect("--hsp3 引数は省略できません。"));
        crate::lsp_server::lsp_main::start_lsp_server(hsp_root)
    }
}

pub fn main() {
    match switch_on_args(std::env::args_os()) {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{:?}", err);
            std::process::exit(1)
        }
    }
}
