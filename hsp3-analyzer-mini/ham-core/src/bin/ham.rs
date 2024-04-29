// HAM CLI: HAMの機能をコマンドラインアプリとして提供する (予定)

// 開発環境での起動方法:
//
// ```sh
// HSP3_ROOT=C:/.../hsp3x cargo run --bin ham
// ```

use ham_core::{
    start_lsp_server,
    subcommands::{self, format_comments::format_comments},
};
use std::{
    fs,
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
};

fn get_help() -> String {
    format!(
        r#"ham {version}

    USAGE: ham [OPTIONS] [SUBCOMMAND]

    EXAMPLE: ham --hsp "C:/hsp36" profile-parse

    SUBCOMMANDS:
        lsp
            LSPサーバーとして起動する (標準入出力でメッセージを送受信する)
            (HSPインストールディレクトリの指定が必須)
            ENV: HAM_LINT=1   リントを有効化する

        parse [FILES...]

        profile-parse
            (HSPインストールディレクトリの指定が必須)

        format-comments [FILES]
            (**注意**: ファイルは上書きされます。必ずバックアップしてください)
            HSPのスクリプトのコメントを // 形式から ; 形式に変更し、
            入力スクリプトファイルを上書きします

    OPTIONS:
        -h, --help      Print help
        -V, --version   Print Version
            --hsp       HSPインストールディレクトリを指定

    ENV:
        HSP3_ROOT       HSPインストールディレクトリを指定 (--hsp より優先度低)
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

static SUBCOMMANDS: &'static [&'static str] = &[
    "format-comments",
    "lsp",
    "parse",
    "profile-parse",
    "help",
    "version",
];

static ERROR_HSP3_ROOT_MISSING: &'static str = r#"HSPのインストールディレクトリを指定してください。(例: コマンドライン引数に --hsp "C:/hsp36" のように指定する、あるいは環境変数 HSP3_ROOT にパスを指定する)"#;

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
        "format-comments" => {
            let mut count = 0;
            for arg in args {
                if arg.starts_with("-") && arg != "-" {
                    panic!("ERROR: Unknown argument: {arg:?}");
                }
                if arg == "-" {
                    let mut buf = String::with_capacity(4096);
                    stdin().read_to_string(&mut buf).unwrap();
                    let output = format_comments(&buf);
                    stdout().write_all(output.as_bytes()).unwrap();
                } else {
                    let filename = arg;
                    let contents = fs::read_to_string(&filename).expect("read");
                    let output = format_comments(&contents);
                    fs::write(&filename, &output).expect("write");
                }
                count += 1;
            }
            if count == 0 {
                eprintln!("ERROR: 入力ファイルが出力されていません");
            }
            return;
        }
        "lsp" => {
            // require root
            let hsp3_root = PathBuf::from(
                hsp3_root_opt
                    .or_else(|| std::env::var("HSP3_ROOT").ok())
                    .expect(ERROR_HSP3_ROOT_MISSING),
            );
            if !hsp3_root.is_dir() {
                panic!("HSP3_ROOTディレクトリがみつかりません: {hsp3_root:?}");
            }

            // halt args
            if let Some(arg) = args.next() {
                panic!("ERROR: Unrecognized argument: {arg:?}");
            }

            start_lsp_server(hsp3_root);
            return;
        }
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

            subcommands::parse::parse_subcommand(files);
        }
        "profile-parse" => {
            // require root
            let hsp3_root = PathBuf::from(
                hsp3_root_opt
                    .or_else(|| std::env::var("HSP3_ROOT").ok())
                    .expect(ERROR_HSP3_ROOT_MISSING),
            );
            if !hsp3_root.is_dir() {
                panic!("HSP3_ROOTディレクトリがみつかりません: {hsp3_root:?}");
            }

            // halt args
            if let Some(arg) = args.next() {
                panic!("ERROR: Unrecognized argument: {arg:?}");
            }

            subcommands::profile_parse::profile_parse_subcommand(hsp3_root);
        }
        arg => unreachable!("arg={arg:?}"),
    }
}
