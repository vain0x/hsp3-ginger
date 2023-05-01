use std::{
    env,
    ffi::OsStr,
    fs,
    io::{stdin, Read, Write},
    path::PathBuf,
    process::{self, Command},
    thread,
};

enum Msg {
    AdapterExited,
    Write(String),
    Closing,
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

    // debug adapter process
    let mut ap = Command::new(&middle_adapter_exe)
        .current_dir(&workspace_dir)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .expect("middle-adapter.exe spawn");

    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    let tx = &tx;

    thread::scope(move |scope| {
        let mut ap_stdin = ap.stdin.take().unwrap();
        let mut ap_stdout = ap.stdout.take().unwrap();

        // wait for child process
        scope.spawn({
            let tx = &*tx;
            move || {
                match ap.wait() {
                    Ok(code) => {
                        eprintln!("ap exited with {code}");
                    }
                    Err(err) => {
                        eprintln!("ap wait failed {err:?}");
                    }
                };
                tx.send(Msg::AdapterExited).unwrap();
            }
        });

        // handle stdin
        scope.spawn({
            let tx = &*tx;
            move || {
                let mut buf = [0; 1024];
                loop {
                    match stdin().read(&mut buf) {
                        Ok(n) => {
                            eprintln!("main: stdin read {n}");
                            continue;
                        }
                        Err(err) => {
                            eprintln!("main: stdin err {err:?}");
                            break;
                        }
                    };
                }
                tx.send(Msg::Closing).unwrap();
            }
        });

        // write thread
        scope.spawn({
            let tx = &*tx;
            move || {
                for msg in rx {
                    match msg {
                        Msg::AdapterExited => {
                            eprintln!("main: adapter exited");
                            return;
                        }
                        Msg::Write(msg) => {
                            eprintln!("main: writing to");
                            write!(ap_stdin, "Content-Length: {}\n\n{}", msg.len(), &msg).unwrap();
                        }
                        Msg::Closing => {
                            eprintln!("write disconnect");
                            let disconnect_req =
                                r#"{{"type": "request", "command": "disconnect", "seq": 9999}}"#
                                    .to_string();
                            tx.send(Msg::Write(disconnect_req)).unwrap();
                        }
                    }
                }
            }
        });

        // handle ap.stdout
        scope.spawn({
            // let tx = &*tx;
            move || {
                let mut buf = [0; 4096];
                let mut out = String::new();
                loop {
                    match ap_stdout.read(&mut buf) {
                        Ok(0) => {
                            eprintln!("main: ap.stdout closed");
                            return;
                        }
                        Ok(n) => {
                            out.clear();
                            out += &String::from_utf8_lossy(&buf[0..n]);
                            eprintln!("main: ap.stdout read {n}: ``{out}``");
                            continue;
                        }
                        Err(err) => {
                            eprintln!("main: ap.stdout err {err:?}");
                            return;
                        }
                    };
                }
            }
        });

        // init
        eprintln!("write initialize");
        let mut seq = 0;
        let initialize_req = {
            seq += 1;
            format!(
                r#"{{"type": "request", "command": "initialize", "seq": {}}}"#,
                seq
            )
        };
        tx.send(Msg::Write(initialize_req)).unwrap();

        eprintln!("write launch");
        let launch_req = {
            seq += 1;

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
    });
}
