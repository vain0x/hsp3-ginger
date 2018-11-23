//! ミドルアダプター。デバッグアダプターの実装。
//! VSCode から直接起動されて、標準入出力経由で通信する。
//!
//! ### 責務
//! 1. デバッグアダプタープロトコルのうち、InitializeRequest から
//!     LaunchRequest へのレスポンスまで処理する。
//! 2. LaunchRequest のレスポンスを返した後、TCP サーバーを起動して、
//!     HSP ランタイムと接続しているはずの adapter と接続する。
//! 3. LaunchRequest の引数として渡される情報を adapter に引き渡す。
//! 4. TCP での通信を標準入出力に転送して、adapter と VSCode が通信できるようにする。

extern crate env_logger;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use debug_adapter_connection::{self as dac, Logger};
use debug_adapter_protocol as dap;
use std::io::{Read, Write};
use std::{fmt, io, net, path, process, thread};

#[allow(unused)]
mod debug_adapter_protocol {
    include!("../../adapter/src/debug_adapter_protocol.rs");
}

#[allow(unused)]
mod debug_adapter_connection {
    include!("../../adapter/src/debug_adapter_connection.rs");
}

struct BeforeLaunchHandler<L: Logger> {
    r: dac::DebugAdapterReader<io::BufReader<io::Stdin>, L>,
    w: dac::DebugAdapterWriter<io::Stdout, L>,
    l: L,
    body: Vec<u8>,
}

enum Status {
    Launch { args: dap::LaunchRequestArgs },
    Disconnect,
}

impl<L: Logger> BeforeLaunchHandler<L> {
    fn log(&self, args: fmt::Arguments) {
        self.l.log(args);
    }

    fn send<T: serde::Serialize>(&mut self, obj: &T) {
        self.w.write(obj);
    }

    fn handle(&mut self, request: &serde_json::Value) -> Option<Status> {
        let request = request.as_object()?;
        let request_seq = request.get("seq")?.as_i64()?;
        let command = request.get("command")?.as_str()?;

        match command {
            "initialize" => self.send(&dap::Msg::Response {
                request_seq,
                success: true,
                e: dap::Response::Initialize,
            }),
            "launch" => {
                self.send(&dap::Msg::Response {
                    request_seq,
                    success: true,
                    e: dap::Response::Launch,
                });
                let args = serde_json::from_value(request.get("arguments")?.clone()).ok()?;
                return Some(Status::Launch { args });
            }
            _ => {
                self.log(format_args!(
                    "コマンドを認識できませんでした {}",
                    command
                ));
                return None;
            }
        }

        None
    }

    fn run(&mut self) -> Status {
        loop {
            if !self.r.recv(&mut self.body) {
                return Status::Disconnect;
            }

            let message = serde_json::from_slice(&self.body).unwrap();
            match self.handle(&message) {
                None => continue,
                Some(result) => return result,
            }
        }
    }
}

struct AfterLaunchHandler<L> {
    #[allow(unused)]
    l: L,
    args: dap::LaunchRequestArgs,
    stream: Option<net::TcpStream>,
}

impl<L: Logger> AfterLaunchHandler<L> {
    fn run(&mut self) {
        let mut in_stream = self.stream.take().unwrap();
        let mut out_stream = in_stream.try_clone().unwrap();

        eprintln!("オプションを送信します");

        {
            let mut w = dac::DebugAdapterWriter::new(&mut out_stream, self.l.clone());
            w.write(&dap::Msg::Request {
                seq: 2,
                e: dap::Request::Options {
                    args: self.args.clone(),
                },
            });
        }

        eprintln!("通信の中継を開始します");

        let j1 = thread::spawn(move || {
            let mut stdin = io::stdin();
            let mut buf = vec![0; 1000];
            let mut go = || loop {
                let n = stdin.read(&mut buf)?;
                if n == 0 {
                    return Ok(());
                }
                out_stream.write_all(&buf[0..n])?;
                out_stream.flush()?;
            };
            go().map_err(|e: io::Error| eprintln!("ERROR stdin → TCP {:?}", e))
                .ok();
        });

        let j2 = thread::spawn(move || {
            let mut buf = vec![0; 1000];
            let mut stdout = io::stdout();
            let mut go = || loop {
                let n = in_stream.read(&mut buf)?;
                if n == 0 {
                    return Ok(());
                }
                stdout.write_all(&buf[0..n])?;
                stdout.flush()?;
            };
            go().map_err(|e: io::Error| eprintln!("ERROR (TCP→stdout) {:?}", e))
                .ok();
        });

        eprintln!("TCPの中継の終了を待機します");

        j1.join().ok();
        j2.join().ok();

        eprintln!("終了を通知します");

        {
            let mut w = dac::DebugAdapterWriter::new(io::stdout(), self.l.clone());
            w.write(&dap::Msg::Event {
                e: dap::Event::Terminated { restart: false },
            });
        }
    }
}

struct Program<L> {
    l: L,
}

impl<L: Logger> Program<L> {
    fn log(&self, args: fmt::Arguments) {
        self.l.log(args);
    }

    fn run(&self) {
        self.log(format_args!("init"));

        let result = BeforeLaunchHandler {
            r: dac::DebugAdapterReader::new(io::BufReader::new(io::stdin()), self.l.clone()),
            w: dac::DebugAdapterWriter::new(io::stdout(), self.l.clone()),
            l: self.l.clone(),
            body: Vec::new(),
        }.run();

        let args = match result {
            Status::Disconnect => {
                self.log(format_args!(
                    "プログラムの起動前に切断されました"
                ));
                return;
            }
            Status::Launch { args } => args,
        };

        self.log(format_args!("引数: {:?}", args));

        self.log(format_args!("TCP接続を開始します"));

        let port = 57676;
        let listener =
            net::TcpListener::bind(net::SocketAddr::from(([127, 0, 0, 1], port))).unwrap();

        // thread::spawn(move || {
        //     let mut stream = net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        //     let mut buf = vec![0; 1000];
        //     loop {
        //         match stream.read(&mut buf) {
        //             Ok(0) => break,
        //             Ok(n) => {
        //                 stream.write_all(&buf[0..n]).unwrap();
        //                 continue;
        //             }
        //             Err(_) => panic!(),
        //         }
        //     }
        // });

        self.log(format_args!("デバッグ実行を開始します"));

        let comp_path = path::PathBuf::from(args.root.clone()).join("cHspComp.exe");
        let mut child = match process::Command::new(comp_path)
            .arg("/diw")
            .arg(args.program.clone())
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .spawn()
        {
            Err(err) => {
                eprintln!("デバッグ実行を開始できません {:?}", err);
                return;
            }
            Ok(child) => child,
        };

        self.log(format_args!("TCPへの接続を待機します"));

        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) => {
                self.log(format_args!("TCP接続が来ませんでした {:?}", e));
                return;
            }
        };

        AfterLaunchHandler {
            args,
            stream: Some(stream),
            l: self.l.clone(),
        }.run();

        // eprintln!("10秒後にデバッグ実行を停止します");
        // thread::sleep(std::time::Duration::from_secs(10));
        child.kill().ok();

        self.log(format_args!("exit"));
    }

    fn new(l: L) -> Self {
        Program { l }
    }
}

fn main() {
    #[cfg(debug_assertions)]
    {
        let logger = {
            let log_path = std::env::temp_dir().join("hsp3-debug-ginger-middle-adapter-log.txt");
            eprintln!("{:?}", log_path);
            dac::FileLogger::create(&log_path)
        };
        Program::new(logger).run();
    }

    #[cfg(not(debug_assertions))]
    {
        let logger = dac::NullLogger;
        Program::new(logger).run();
    }
}
