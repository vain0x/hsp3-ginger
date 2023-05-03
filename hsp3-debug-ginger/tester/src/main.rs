// デバッグアダプターの実行を試すスクリプト

// (DAPにおける開発ツールとして動作する。すなわちVSCodeの代わりにデバッグセッションを開始する機能を持つ。
//  動作確認のためにVSCodeを起動するのはめんどうなため)

use std::{
    env,
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{self, Command},
    sync::{
        atomic::{self, AtomicU32},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

/// スレッド間のメッセージ
enum Msg {
    /// アダプタープロセスの終了が検出されたとき
    AdapterExited,
    /// キャンセル (testerの実行中止) が要求されたとき
    CancelRequested,

    /// アダプターにメッセージを書き込むこと
    Write(String),
}

fn main() {
    // <path-to>/tester
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(manifest_dir.file_name(), Some(OsStr::new("tester")));

    // <path-to>/hsp3-debug-ginger
    let workspace_dir = {
        let mut p = manifest_dir.to_owned();
        p.pop();
        p
    };
    assert_eq!(
        workspace_dir.file_stem(),
        Some(OsStr::new("hsp3-debug-ginger"))
    );

    // HSP3_ROOT
    let hsp3_root = PathBuf::from(env::var("HSP3_ROOT").unwrap());
    eprintln!("hsp3_root={:?}", hsp3_root);

    // $HSP3_ROOT/hsp3.exe が存在すること
    assert!({
        let mut p = hsp3_root.to_owned();
        p.push("hsp3.exe");
        fs::metadata(&p).unwrap().is_file()
    });

    // middle-adapter
    let middle_adapter_exe = {
        let mut p = manifest_dir.to_owned();
        p.pop();
        p.extend("target/debug/middle-adapter.exe".split('/'));
        p
    };
    eprintln!("middle-adapter={:?}", middle_adapter_exe);

    // デバッグアダプターのプロセスを起動する
    let mut ap = Command::new(&middle_adapter_exe)
        .current_dir(&workspace_dir)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .expect("middle-adapter spawn");

    let mut ap_stdin = ap.stdin.take().unwrap();
    let mut ap_stdout = ap.stdout.take().unwrap();

    // 複数の処理を並行に行うため、複数のスレッドを起動する
    //
    // - スレッド同士はメッセージ通信によって協調動作する (txで送信, rxで受信)
    // - (Graceful shutdown) すべてのスレッドがtx, rxを破棄すると全スレッドが処理を完了し、testerが終了できる
    let (tx, rx) = mpsc::sync_channel(1);
    let tx = Arc::new(tx);
    let seq = AtomicU32::new(1);

    let (handler_tx, handler_rx) = mpsc::sync_channel(1);
    let handler_state = Arc::new(Mutex::new(Some((Arc::clone(&tx), handler_rx))));

    // `Ctrl+C` の入力時の処理を登録する
    ctrlc::set_handler({
        let handler_state = Arc::clone(&handler_state);

        move || {
            eprintln!("tester: On Ctrl+C");

            let mut state = handler_state.lock().unwrap();
            let state = match state.take() {
                Some(it) => it,
                None => {
                    // 2回目のCtrl+C
                    process::exit(1);
                }
            };

            let (tx, handler_rx) = state;
            tx.send(Msg::CancelRequested).unwrap_or_else(|err| {
                eprintln!("tester: Error {err:?}");
            });
            drop(tx);

            // スレッドがすべて停止するまでプロセスの終了をブロックする
            handler_rx.recv_timeout(Duration::from_secs(3)).ok();
        }
    })
    .unwrap();

    thread::scope(|scope| {
        // アダプタープロセスの終了を待つスレッド
        scope.spawn({
            let tx = Arc::clone(&tx);
            move || {
                match ap.wait() {
                    Ok(code) => {
                        eprintln!("tester.wait: adapter exited with {code}");
                    }
                    Err(err) => {
                        eprintln!("tester.wait: adapter wait failed {err:?}");
                    }
                };
                tx.send(Msg::AdapterExited).unwrap_or_else(|err| {
                    eprintln!("tester.wait: Send {err:?}");
                });
            }
        });

        // メッセージを処理するスレッド
        scope.spawn({
            let tx = Arc::clone(&tx);
            move || {
                for msg in rx {
                    match msg {
                        Msg::AdapterExited => {
                            eprintln!("tester.main: On AdapterExited");
                            return;
                        }
                        Msg::CancelRequested => {
                            eprintln!("tester.main: On CancelRequested");

                            // #sendDisconnectedReq
                            let disconnect_req =
                                r#"{{"type": "request", "command": "disconnect", "seq": 9999}}"#
                                    .to_string();

                            tx.send(Msg::Write(disconnect_req)).unwrap_or_else(|err| {
                                eprintln!("tester.main: Error {err:?}");
                            });
                        }
                        Msg::Write(msg) => {
                            eprintln!("tester.main: On Write({})", msg.len());
                            write!(ap_stdin, "Content-Length: {}\r\n\r\n{}", msg.len(), &msg)
                                .unwrap_or_else(|err| {
                                    eprintln!("tester.main: Error {err:?}");
                                });
                        }
                    }
                }
            }
        });

        // アダプターの標準出力から読み取りを行うスレッド
        scope.spawn({
            let seq = &seq;
            let tx = Arc::clone(&tx);
             move || {
                let mut buf = [0; 4096];
                loop {
                    match ap_stdout.read(&mut buf) {
                        Ok(0) => {
                            eprintln!("tester.stdout: アダプターの標準出力が閉じられました");
                            return;
                        }
                        Ok(n) => {
                            let msg = String::from_utf8_lossy(&buf[0..n]);
                            eprintln!("tester.stdout: 読み込み({n}): ``{msg}``");

                            if msg.contains(r#""initialized""#) {
                                eprintln!("tester: On initialized event");
                                eprintln!("write configuration done");
                                let configuration_done_req = {
                                    let seq = seq.fetch_add(1, atomic::Ordering::AcqRel);
                                    format!(
                                        r#"{{"type": "request", "command": "configurationDone", "seq": {}}}"#,
                                        seq
                                    )
                                };
                                tx.send(Msg::Write(configuration_done_req)).unwrap();

                                eprintln!("write launch");
                                let launch_req = {
                                    let seq = seq.fetch_add(1, atomic::Ordering::AcqRel);

                                    let mut p = workspace_dir.to_owned();
                                    p.extend("adapter/tests/hsp/main.hsp".split('/'));

                                    format!(
                                        r#"{{"type": "request", "command": "launch", "seq": {}, "arguments": {{"cwd": "{}", "root": "{}", "program": "{}", "trace": true}}}}"#,
                                        seq,
                                        workspace_dir.to_string_lossy().replace('\\', "/"),
                                        hsp3_root.to_string_lossy().replace('\\', "/"),
                                        p.to_string_lossy().replace('\\', "/")
                                    )
                                };
                                tx.send(Msg::Write(launch_req)).unwrap();
                            }

                            if msg.contains(r#""terminated""#) {
                                eprintln!("tester.stdout: On terminated event");

                                let disconnect_req =
                                    r#"{{"type": "request", "command": "disconnect", "seq": 9999}}"#
                                    .to_string();

                                tx.send(Msg::Write(disconnect_req)).unwrap_or_else(|err| {
                                    eprintln!("tester.main: Error {err:?}");
                                });
                            }
                            continue;
                        }
                        Err(err) => {
                            eprintln!("tester.stdout: Error {err:?}");
                            return;
                        }
                    };
                }
            }
        });

        // デバッグセッションの開始時の処理:
        {
            eprintln!("write initialize");
            let initialize_req = {
                let seq = seq.fetch_add(1, atomic::Ordering::AcqRel);
                format!(
                    r#"{{"type": "request", "command": "initialize", "seq": {}}}"#,
                    seq
                )
            };
            tx.send(Msg::Write(initialize_req)).unwrap();
            drop(tx);
        }
    });

    // キャンセルハンドラーの終了待ちを完了させる (待機していなかったら何も起きない)
    handler_tx.send(()).ok();
}
