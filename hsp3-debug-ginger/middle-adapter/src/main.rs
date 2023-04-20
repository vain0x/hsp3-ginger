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

use log::{debug, error, info, warn};
use shared::{debug_adapter_connection as dac, debug_adapter_protocol as dap};
use std::{
    io::{self, Read, Write},
    net, path, process, thread,
};

struct BeforeLaunchHandler {
    r: dac::DebugAdapterReader<io::BufReader<io::Stdin>>,
    w: dac::DebugAdapterWriter<io::Stdout>,
    body: Vec<u8>,
}

enum Status {
    Launch {
        seq: i64,
        args: dap::LaunchRequestArgs,
    },
    Disconnect,
}

impl BeforeLaunchHandler {
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
                return Some(Status::Launch {
                    seq: request_seq,
                    args,
                });
            }
            _ => {
                warn!("コマンドを認識できませんでした {:?}", command);
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

struct AfterLaunchHandler {
    launch_seq: i64,
    args: dap::LaunchRequestArgs,
    stream: Option<net::TcpStream>,
}

impl AfterLaunchHandler {
    fn run(&mut self) {
        let mut in_stream = self.stream.take().unwrap();
        let mut out_stream = in_stream.try_clone().unwrap();

        info!("オプションを送信します");

        {
            let mut w = dac::DebugAdapterWriter::new(&mut out_stream);
            w.write(&dap::Msg::Request {
                seq: self.launch_seq,
                e: dap::Request::Launch {
                    args: self.args.clone(),
                },
            });
        }

        info!("通信の中継を開始します");

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
            go().map_err(|e: io::Error| error!("ERROR stdin → TCP {:?}", e))
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
            go().map_err(|e: io::Error| error!("ERROR (TCP→stdout) {:?}", e))
                .ok();
        });

        info!("TCPの中継の終了を待機します");

        j1.join().ok();
        j2.join().ok();

        info!("終了を通知します");

        {
            let mut w = dac::DebugAdapterWriter::new(io::stdout());
            w.write(&dap::Msg::Event {
                e: dap::Event::Terminated { restart: false },
            });
        }
    }
}

struct Program;

impl Program {
    fn run(&self) {
        debug!("init");

        let result = BeforeLaunchHandler {
            r: dac::DebugAdapterReader::new(io::BufReader::new(io::stdin())),
            w: dac::DebugAdapterWriter::new(io::stdout()),
            body: Vec::new(),
        }
        .run();

        let (launch_seq, args) = match result {
            Status::Disconnect => {
                info!("プログラムの起動前に切断されました");
                return;
            }
            Status::Launch { seq, args } => (seq, args),
        };

        debug!("引数: {:?}", args);

        info!("TCP接続を開始します");

        let port = 57676;
        let listener =
            net::TcpListener::bind(net::SocketAddr::from(([127, 0, 0, 1], port))).unwrap();

        thread::spawn(move || {
            let mut stream = net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            let mut buf = vec![0; 1000];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        stream.write_all(&buf[0..n]).unwrap();
                        continue;
                    }
                    Err(_) => panic!(),
                }
            }
        });

        info!("デバッグ実行を開始します");

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
                error!("デバッグ実行を開始できません {:?}", err);
                return;
            }
            Ok(child) => child,
        };

        info!("TCPへの接続を待機します");

        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) => {
                error!("TCP接続が来ませんでした {:?}", e);
                return;
            }
        };

        AfterLaunchHandler {
            launch_seq,
            args,
            stream: Some(stream),
        }
        .run();

        // eprintln!("10秒後にデバッグ実行を停止します");
        // thread::sleep(std::time::Duration::from_secs(10));
        child.kill().ok();

        info!("終了");
    }
}

fn init_logger() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::new().filter(None, log_level).init();
}

fn main() {
    init_logger();
    Program {}.run();
}
