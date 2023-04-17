use std::io::{Read, Write};

fn print_help() {
    println!(
        r#"{} v{version}

**注意**: うまく動作する保証はありません！
    必ずバックアップがあることを確認し、
    処理前後のスクリプトの差分を検証してください

HSPのスクリプトのコメントのスタイルを変更します。
スクリプトのエンコーディングはUTF-8である必要があります。

EXAMPLE:
(1)
    {name} a.hsp b.hsp c.hsp

(2)
    cat input.hsp | {name} - >output.hsp"#,
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let mut count = 0;
    for arg in std::env::args().skip(1) {
        if arg == "-" {
            let mut buf = String::with_capacity(4096);
            std::io::stdin().read_to_string(&mut buf).unwrap();
            let buf = ham_core::rewrite_fn(buf);
            std::io::stdout().write_all(buf.as_bytes()).unwrap();
            count += 1;
            continue;
        }

        if arg.starts_with("-") {
            match arg.as_ref() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                _ => {}
            }
            eprintln!("不明なパラメータです。'{}'", arg);
            std::process::exit(1);
        }

        {
            let filename = arg;
            let contents = std::fs::read_to_string(&filename).expect("read file");
            let contents = ham_core::rewrite_fn(contents);
            std::fs::write(&filename, &contents).expect("write file");
            count += 1;
        }
    }

    if count == 0 {
        print_help();
    }
}
