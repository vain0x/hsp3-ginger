use ham_core::start_lsp_server;
use std::{env::ArgsOs, path::PathBuf};

enum Arg {
    Help,
    Version,
    Lsp,
}

fn get_help() -> String {
    format!(
        r#"ham-lsp {version}
    USAGE:
        ham-lsp [OPTIONS] [SUBCOMMAND]

    EXAMPLE:
        ham-lsp --hsp "C:/hsp3" lsp

    SUBCOMMANDS:
        lsp     Start LSP server via STDIN.

    OPTIONS:
        -h, --help      Print help
        -V, --version   Print Version
            --hsp       HSP インストールディレクトリ (必須)

    ENV:
        HAM_LINT=1      リントを有効化する (既定: 無効)"#,
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

fn parse_args(args: ArgsOs) -> Result<Arg, String> {
    let mut args = args.into_iter();
    let verb = match args.next() {
        None => return Ok(Arg::Help),
        Some(verb) => verb,
    };

    match verb.into_string().unwrap().as_str() {
        "-h" | "--help" | "help" => Ok(Arg::Help),
        "-V" | "--version" | "version" => Ok(Arg::Version),
        "lsp" => Ok(Arg::Lsp),
        verb => Err(format!("Unknown subcommand '{}'.", verb)),
    }
}

fn switch_on_args(mut args: ArgsOs) {
    // Skip self path.
    args.next();

    args.next()
        .filter(|a| a == "--hsp")
        .expect("Expected --hsp");

    let hsp3_home = PathBuf::from(args.next().unwrap());

    let arg = parse_args(args).unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit_with_help();
    });

    match arg {
        Arg::Version => exit_with_version(),
        Arg::Help => exit_with_help(),
        Arg::Lsp => start_lsp_server(hsp3_home),
    }
}

fn main() {
    switch_on_args(std::env::args_os())
}
