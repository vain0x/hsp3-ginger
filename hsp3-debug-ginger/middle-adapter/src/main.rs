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

mod pipe;

// use crate::pipe::Pipe;
use log::{debug, error, info, warn};
use named_pipe::{ConnectingServer, PipeClient, PipeOptions, PipeServer};
use shared::{
    debug_adapter_connection as dac,
    debug_adapter_protocol::{self as dap, LaunchRequestArgs},
    file_logger::FileLogger,
};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
    thread,
};
// use winapi::um::winbase::WaitNamedPipeA;

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
        debug!("BLH: handle");
        let request = request.as_object()?;
        let request_seq = request.get("seq")?.as_i64()?;
        let command = request.get("command")?.as_str()?;
        debug!("  command={}", command);

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
                let args: LaunchRequestArgs =
                    serde_json::from_value(request.get("arguments").unwrap().clone()).unwrap();
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
            debug!("BLH: Receiving...");
            if !self.r.recv(&mut self.body) {
                debug!("BLH: result=Disconnect");
                return Status::Disconnect;
            }

            let message = serde_json::from_slice(&self.body).unwrap();
            match self.handle(&message) {
                None => continue,
                Some(result) => {
                    debug!("BLH: return");
                    return result;
                }
            }
        }
    }
}

struct AfterLaunchHandler {
    launch_seq: i64,
    args: dap::LaunchRequestArgs,
    // stream: Option<net::TcpStream>,
    // stream: Option<Pipe>,
    stream: Option<PipeServer>,
}

impl AfterLaunchHandler {
    fn run(&mut self) {
        let stream = self.stream.take().unwrap();
        // let mut out_stream = in_stream.try_clone().unwrap();
        let stream = Arc::new(Mutex::new(stream));

        info!("オプションを送信します");

        {
            let mut out_stream = stream.lock().unwrap();
            let mut w = dac::DebugAdapterWriter::new(&mut *out_stream);
            w.write(&dap::Msg::Request {
                seq: self.launch_seq,
                e: dap::Request::Launch {
                    args: self.args.clone(),
                },
            });
        }

        info!("通信の中継を開始します");

        let s1 = stream.clone();
        let j1 = thread::spawn(move || {
            let mut stdin = io::stdin();
            let mut buf = vec![0; 4096];
            let mut go = || loop {
                let n = stdin.read(&mut buf)?;
                if n == 0 {
                    return Ok(());
                }

                {
                    let mut out_stream = s1.lock().unwrap();
                    out_stream.write_all(&buf[0..n])?;
                    out_stream.flush()?;
                }
            };
            go().map_err(|e: io::Error| error!("[j1] {:?}", e)).ok();
        });

        let s2 = stream.clone();
        let j2 = thread::spawn(move || {
            let in_stream = s2.clone();

            let mut buf = vec![0; 4096];
            let mut stdout = io::stdout();
            let mut go = || loop {
                let mut in_stream = in_stream.lock().unwrap();
                let n = in_stream.read(&mut buf)?;
                if n == 0 {
                    return Ok(());
                }
                stdout.write_all(&buf[0..n])?;
                stdout.flush()?;
            };
            go().map_err(|e: io::Error| error!("[j2] {:?}", e)).ok();
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

        let ax_file = "examples/inc_loop.ax";

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

        // info!("TCP接続を開始します");

        // let port = 57676;
        // let listener =
        //     net::TcpListener::bind(net::SocketAddr::from(([127, 0, 0, 1], port))).unwrap();

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

        info!("パイプを作成します");
        // let stream = Pipe::new(r"\\.\pipe\hdg-pipe");
        let stream = PipeOptions::new(r"\\.\pipe\hdg-pipe").single().unwrap();
        // .multiple(2)
        // .expect("multiple");

        info!("デバッグ実行を開始します");

        let rt_file = PathBuf::from(format!("{}/hsp3.exe", &args.root));

        // let comp_path = path::PathBuf::from(args.root.clone()).join("cHspComp.exe");
        // let mut child = match process::Command::new(comp_path)
        //     .arg("/diw")
        //     .arg(args.program.clone())
        //     .stdin(process::Stdio::null())
        //     .stdout(process::Stdio::null())
        //     .stderr(process::Stdio::null())
        //     .spawn()
        // {
        //     Err(err) => {
        //         error!("デバッグ実行を開始できません {:?}", err);
        //         return;
        //     }
        //     Ok(child) => child,
        // };
        // let comp_path = path::PathBuf::from(args.root.clone()).join("cHspComp.exe");
        let mut child = match process::Command::new(rt_file)
            .arg(ax_file)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .env("RUST_BACKTRACE", "1")
            .spawn()
        {
            Err(err) => {
                error!("デバッグ実行を開始できません {:?}", err);
                return;
            }
            Ok(child) => child,
        };

        // WaitNamedPipe
        // debug!("Waiting pipes");
        // {
        //     let pipe_name = concat!(r"\\.\pipe\hdg-pipe", "\0");
        //     let ok = unsafe { WaitNamedPipeA(pipe_name.as_ptr() as *const i8, 1000u32) } != 0;
        //     debug!("wait={}", ok);
        // }

        // info!("TCPへの接続を待機します");

        // let stream = match listener.accept() {
        //     Ok((stream, _)) => stream,
        //     Err(e) => {
        //         error!("TCP接続が来ませんでした {:?}", e);
        //         return;
        //     }
        // };

        debug!("ランタイム側のパイプを待機しています");
        let stream = stream.wait().unwrap();

        AfterLaunchHandler {
            launch_seq,
            args,
            stream: Some(stream),
        }
        .run();

        info!("子プロセスをキルします");
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
        log::LevelFilter::Info
    };

    let logger = FileLogger::new(&PathBuf::from("middle-adapter.log")).expect("logger");
    log::set_max_level(log_level);
    log::set_boxed_logger(Box::new(logger)).expect("set_logger");
    info!("Log is set with level={:?}", log_level);
    // env_logger::Builder::new().filter(None, log_level).init();
}

fn main() {
    init_logger();
    Program {}.run();
}
