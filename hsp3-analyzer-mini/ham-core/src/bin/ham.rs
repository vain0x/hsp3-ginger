// HAM CLI: HAMの機能をコマンドラインアプリとして提供する (予定)

// 開発環境での起動方法:
//
// ```sh
// HSP3_ROOT=C:/.../hsp3x cargo run --bin ham
// ```

use ham_core::commands;
use std::path::PathBuf;

fn get_help() -> String {
    format!(
        r#"ham {version}
    USAGE:
        ham [OPTIONS] [SUBCOMMAND]

    EXAMPLE:
        ham --hsp "C:/hsp36" profile-parse

    SUBCOMMANDS:
        parse [FILES...]
        profile-parse

    OPTIONS:
        -h, --help      Print help
        -V, --version   Print Version
            --hsp       HSP インストールディレクトリ (必須)
"#,
        version = get_version()
    )
}

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn exit_with_help() -> ! {
    eprintln!("{}", get_help());
    std::process::exit(0)
}

fn exit_with_version() -> ! {
    eprintln!("{}", get_version());
    std::process::exit(0)
}

static SUBCOMMANDS: &'static [&'static str] = &["parse", "profile-parse", "help", "version"];

static ERROR_HSP3_ROOT_MISSING: &'static str = "HSPのインストールディレクトリを指定してください。(例: コマンドライン引数に --hsp C:/hsp36 のように指定する、あるいは環境変数 HSP3_ROOT にパスを指定する)";

fn main() {
    let mut args = std::env::args();

    // Skip self path.
    args.next();

    let mut subcommand_opt = None;
    let mut hsp3_root_opt = None;

    while let Some(arg) = args.next() {
        if arg.starts_with("-") && arg != "-" {
            match arg.as_str() {
                "-h" | "--help" => exit_with_help(),
                "-V" | "--version" => exit_with_version(),
                "--hsp" => {
                    let value = args.next().expect("--hsp value");
                    hsp3_root_opt = Some(value.to_string());
                }
                _ => {
                    eprintln!("ERROR: Unrecognized option: {arg:?}");
                    std::process::exit(1)
                }
            }
            continue;
        }

        if subcommand_opt.is_none() {
            if !SUBCOMMANDS.contains(&arg.as_str()) {
                eprintln!("ERROR: Unrecognized subcommand: {arg:?}");
                std::process::exit(1)
            }

            subcommand_opt = Some(arg);
            break;
        }
    }

    match subcommand_opt.unwrap_or_default().as_str() {
        "" | "help" => exit_with_help(),
        "version" => exit_with_version(),
        "parse" => {
            let mut files = vec![];
            for arg in args.into_iter() {
                if arg.starts_with("-") && arg != "-" {
                    panic!("ERROR: Unknown argument: {arg:?}");
                }
                files.push(arg);
            }
            if files.is_empty() {
                panic!("ERROR: 入力ファイルが指定されていません");
            }

            commands::parse(files);
        }
        "profile-parse" => {
            let hsp3_root = PathBuf::from(
                hsp3_root_opt
                    .or_else(|| std::env::var("HSP3_ROOT").ok())
                    .expect(ERROR_HSP3_ROOT_MISSING),
            );
            if !hsp3_root.is_dir() {
                panic!("HSP3_ROOTディレクトリがみつかりません: {hsp3_root:?}");
            }
            if let Some(arg) = args.next() {
                panic!("ERROR: Unrecognized argument: {arg:?}");
            }
            commands::profile_parse(hsp3_root);
        }
        arg => unreachable!("arg={arg:?}"),
    }
}
