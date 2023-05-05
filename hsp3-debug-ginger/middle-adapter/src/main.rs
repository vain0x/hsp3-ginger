//! ミドルアダプター。デバッグアダプターの実装。
//! VSCode から直接起動されて、標準入出力経由で通信する。
//!
//! ### 責務
//! 1. デバッグアダプタープロトコルのうち、InitializeRequest から
//!     LaunchRequest へのレスポンスまで処理する。
//! 2. LaunchRequest のレスポンスを返した後、名前付きパイプを生成して、
//!     HSP ランタイムと接続しているはずの adapter と接続する。
//! 3. LaunchRequest の引数として渡される情報を adapter に引き渡す。
//! 4. 名前付きパイプの通信を標準入出力に転送して、adapter と VSCode が通信できるようにする。

mod hspcmp;
mod pipe;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::pipe::Pipe;
use shared::{
    debug_adapter_connection as dac,
    debug_adapter_protocol::{self as dap, LaunchRequestArgs},
    file_logger::FileLogger,
};
use std::{
    io::{self, Read, Write},
    mem,
    path::PathBuf,
    process,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

enum AMsg {
    DapReaderRecv(serde_json::Value),
    DapReaderExited,
    PipeExited,
    StdOutExited,
    ChildExited,
    Launch { args: LaunchRequestArgs },
    DisconnectSelf,
}

enum WMsg {
    WriteMsg(dap::Msg),
    WriteBytes(Vec<u8>),
    Exit,
}

fn run() {
    let (tx, rx) = mpsc::sync_channel(8);
    let tx = Arc::new(tx);

    debug!("パイプを作成します");
    let mut in_stream = Pipe::new(r"\\.\pipe\hdg-pipe-up");
    let mut out_stream = Pipe::new(r"\\.\pipe\hdg-pipe-down");

    let (stdout_tx, stdout_rx) = mpsc::sync_channel(1);
    let stdout_tx = &stdout_tx;
    let (child_tx, child_rx) = mpsc::sync_channel(0);

    let mut child_opt = None;

    thread::scope(move |scope| {
        // [dap-reader]
        let tx_for_dap_reader = Arc::clone(&tx);
        scope.spawn(move || {
            debug!("[dap-reader] 開始");
            let tx = tx_for_dap_reader;
            let stdin = io::stdin().lock();
            let mut dap_reader = dac::DebugAdapterReader::new(stdin);
            let mut buf = vec![0; 4096];
            let mut launching = false;

            loop {
                if !dap_reader.recv(&mut buf) {
                    debug!("[dap-reader] 標準入力が閉じました");
                    tx.send(AMsg::DapReaderExited).unwrap();
                    return;
                }

                let msg: serde_json::Value = serde_json::from_slice(&buf).unwrap();

                // let request = msg.as_object().unwrap();
                let seq = msg.get("seq").unwrap().as_i64().unwrap();
                let command = msg.get("command").unwrap().as_str().unwrap();
                let mut terminated = false;

                match command {
                    "launch" => launching = true,
                    "terminated" => terminated = true,
                    _ => {}
                }

                debug!("[dap-reader] 読み取りました (seq: {seq}, command: {command})");
                tx.send(AMsg::DapReaderRecv(msg)).unwrap();

                if launching {
                    break;
                }
                if terminated {
                    debug!("[dap-reader] Launchの前に停止しました");
                    return;
                }
            }

            assert!(launching);
            debug!("[dap-reader] クライアント側の下りパイプが開かれるのを待っています");
            out_stream.accept();

            debug!("[dap-reader] Launchリクエストを転送します");
            write!(out_stream, "Content-Length: {}\r\n\r\n", buf.len()).unwrap();
            out_stream.write_all(&buf).unwrap();
            out_stream.flush().unwrap();

            debug!("[dap-reader] Launchリクエストの後");
            let mut stdin = dap_reader.into_inner();
            buf.resize(4096, 0);

            let result = loop {
                let n = match stdin.read(&mut buf) {
                    Ok(0) => {
                        debug!("[dap-reader] 標準入力が閉じました");
                        break Ok(());
                    }
                    Ok(it) => it,
                    Err(err) => break Err(err),
                };
                if let Err(err) = out_stream.write_all(&buf[..n]) {
                    break Err(err);
                }
                if let Err(err) = out_stream.flush() {
                    break Err(err);
                }
            };

            tx.send(AMsg::DapReaderExited).unwrap_or_else(|_| {
                debug!("[dap-reader] tx.send");
            });
            if let Err(err) = result {
                debug!("[dap-reader] {err:?}");
            }
            debug!("[dap-reader] 終了");
        });

        // [pipe-copy]
        let tx_for_pipe_copy = Arc::clone(&tx);
        scope.spawn(move || {
            debug!("[pipe-copy] 開始");
            let tx = tx_for_pipe_copy;
            let mut buf = vec![0; 4096];

            in_stream.accept();
            debug!("[pipe-copy] クライアント側の上りパイプが開かれました");

            loop {
                let n = match in_stream.read(&mut buf) {
                    Ok(0) => {
                        debug!("[pipe-copy] 上りパイプが閉じました");
                        break;
                    }
                    Ok(it) => it,
                    Err(err) => {
                        debug!("[pipe-copy] 読み取り中 {err:?}");
                        break;
                    }
                };
                stdout_tx
                    .send(WMsg::WriteBytes(buf[..n].to_owned()))
                    .unwrap();
            }
            tx.send(AMsg::PipeExited).unwrap_or_else(|_| {
                debug!("[pipe-copy] tx.send");
            });
            debug!("[pipe-copy] 終了");
        });

        // [stdout]
        let tx_for_stdout = Arc::clone(&tx);
        scope.spawn(move || {
            debug!("[stdout] 開始");
            let tx = tx_for_stdout;
            let stdout = io::stdout().lock();
            let mut stdout_opt = Some(stdout);
            let mut buf = Vec::with_capacity(4096);
            for msg in stdout_rx {
                match msg {
                    WMsg::WriteMsg(msg) => {
                        debug!("[stdout] On WriteMsg {:?}", msg);
                        let stdout = stdout_opt.take().unwrap();
                        let local_buf = mem::take(&mut buf);

                        let mut dap_writer =
                            dac::DebugAdapterWriter::with_buffer(stdout, local_buf);
                        dap_writer.write(&msg);
                        let (stdout, local_buf) = dap_writer.into_inner();

                        stdout_opt = Some(stdout);
                        buf = local_buf;
                        buf.clear();
                    }
                    WMsg::WriteBytes(data) => {
                        debug!(
                            "[stdout] On WriteBytes({}B) {:?}",
                            data.len(),
                            String::from_utf8_lossy(&data)
                        );
                        let stdout = stdout_opt.as_mut().unwrap();
                        stdout.write_all(&data).unwrap();
                        stdout.flush().unwrap();
                    }
                    WMsg::Exit => {
                        debug!("[stdout] On Exit");
                        break;
                    }
                }
            }
            tx.send(AMsg::StdOutExited).unwrap_or_else(|_| {
                error!("[stdout] tx.send");
            });
            debug!("[stdout] 終了");
        });

        // [child]
        let tx_for_child = Arc::clone(&tx);
        scope.spawn(move || {
            debug!("[child] 開始");
            let tx = tx_for_child;

            let child_ref: Arc<Mutex<Option<process::Child>>> = child_rx.recv().unwrap();
            debug!("[child] 子プロセスを受け取りました");

            loop {
                thread::sleep(Duration::from_millis(500));
                let mut guard = child_ref.lock().unwrap();
                let child = match guard.as_mut() {
                    Some(it) => it,
                    None => {
                        debug!("[child] キルを検出しました");
                        break;
                    }
                };
                match child.try_wait() {
                    Ok(Some(status)) => {
                        debug!("[child] 終了を検出しました ({status:?})");
                        break;
                    }
                    Ok(None) => continue,
                    Err(err) => {
                        debug!("[child] try_wait {err:?}");
                        break;
                    }
                }
            }
            tx.send(AMsg::ChildExited).unwrap_or_else(|_| {
                error!("[child] tx.send");
            });
            debug!("[child] 終了");
        });

        // [main]
        scope.spawn(move || {
            debug!("[main] 開始");
            for msg in rx {
                match msg {
                    AMsg::DapReaderRecv(msg) => {
                        debug!("[main] On DapReaderRecv");
                        let req = msg.as_object().unwrap();
                        let seq = msg.get("seq").unwrap().as_i64().unwrap();
                        let command = msg.get("command").unwrap().as_str().unwrap();

                        match command {
                            "initialize" => {
                                stdout_tx
                                    .send(WMsg::WriteMsg(dap::Msg::Response {
                                        request_seq: seq,
                                        success: true,
                                        e: dap::Response::Initialize,
                                    }))
                                    .unwrap();

                                stdout_tx
                                    .send(WMsg::WriteMsg(dap::Msg::Event {
                                        e: dap::Event::Initialized,
                                    }))
                                    .unwrap();
                                continue;
                            }
                            "configurationDone" => {
                                stdout_tx
                                    .send(WMsg::WriteMsg(dap::Msg::Response {
                                        request_seq: seq,
                                        success: true,
                                        e: dap::Response::ConfigurationDone,
                                    }))
                                    .unwrap();
                                continue;
                            }
                            "launch" => {
                                stdout_tx
                                    .send(WMsg::WriteMsg(dap::Msg::Response {
                                        request_seq: seq,
                                        success: true,
                                        e: dap::Response::Launch,
                                    }))
                                    .unwrap();

                                let args: LaunchRequestArgs =
                                    serde_json::from_value(req.get("arguments").unwrap().clone())
                                        .unwrap();

                                tx.send(AMsg::Launch { args }).unwrap();
                                continue;
                            }
                            _ => {
                                debug!("[main] \\- 不明なコマンド '{command}'");
                                continue;
                            }
                        }
                    }
                    AMsg::Launch { args } => {
                        debug!("[main] On Launch");

                        // FIXME: args.program に指定されたスクリプトをコンパイル・実行する
                        let rt_file = PathBuf::from(format!("{}/hsp3.exe", &args.root));
                        let ax_file = "examples/inc_loop.ax";

                        let child = process::Command::new(rt_file)
                            .arg(ax_file)
                            .stdin(process::Stdio::null())
                            .stdout(process::Stdio::null())
                            .stderr(process::Stdio::inherit())
                            .env("RUST_BACKTRACE", "1")
                            .spawn()
                            .unwrap_or_else(|err| panic!("デバッグ実行を開始できません {:?}", err));

                        debug!("[main] \\- Sending child (pid: {})", child.id());
                        let child = Arc::new(Mutex::new(Some(child)));
                        child_tx.send(Arc::clone(&child)).unwrap();
                        child_opt = Some(child);
                        continue;
                    }
                    AMsg::DapReaderExited => {
                        debug!("[main] On DapReaderExited");
                        tx.send(AMsg::DisconnectSelf).unwrap();
                    }
                    AMsg::PipeExited => {
                        debug!("[main] On PipeExited");
                        tx.send(AMsg::DisconnectSelf).unwrap();
                    }
                    AMsg::StdOutExited => {
                        debug!("[main] On StdOutExited");
                        tx.send(AMsg::DisconnectSelf).unwrap();
                    }
                    AMsg::ChildExited => {
                        debug!("[main] On ChildExited");
                        tx.send(AMsg::DisconnectSelf).unwrap();
                    }
                    AMsg::DisconnectSelf => {
                        debug!("[main] On DisconnectSelf");

                        // 子プロセスをキルする
                        'killing: {
                            let child = match child_opt.take() {
                                Some(it) => it,
                                None => {
                                    debug!("[main] \\- 子プロセスはすでに終了しているようです");
                                    break 'killing;
                                }
                            };
                            let mut child_opt = match child.lock() {
                                Ok(it) => it,
                                Err(err) => {
                                    debug!("[main] \\- child.lock {err:?}");
                                    break 'killing;
                                }
                            };
                            let mut child = match child_opt.take() {
                                Some(it) => it,
                                None => {
                                    // 起動されていないか、すでに終了している
                                    debug!("[main] \\- No child");
                                    break 'killing;
                                }
                            };
                            child.kill().unwrap_or_else(|err| {
                                warn!("[main] \\- 子プロセスをキルできません {err:?}");
                            });
                        }

                        // 標準出力への書き込みを終了する
                        stdout_tx.send(WMsg::Exit).unwrap();
                        break;
                    }
                }
            }
            debug!("[main] 終了");
        });
    });
    debug!("thread::scope finished");
}

/// ロガーを設定する
///
/// `debug!()`, `info!()`, `error!()` などのマクロによって出力されるログメッセージがファイルに書き込まれるように設定する。
/// 出力先はこのプロセスのカレントディレクトリの `middle-adapter.log`
fn init_logger() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let logger = FileLogger::new(&PathBuf::from("middle-adapter.log")).expect("logger");
    log::set_max_level(log_level);
    log::set_boxed_logger(Box::new(logger)).expect("set_logger");
    debug!("ロガーが設定されました (レベル: {:?})", log_level);
}

fn main() {
    init_logger();
    debug!("=== 開始 ===");
    run();
    debug!("=== 終了 ===");
}
