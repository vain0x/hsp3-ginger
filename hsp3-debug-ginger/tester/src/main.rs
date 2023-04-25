use std::{
    env,
    ffi::OsStr,
    fs,
    io::Write,
    path::PathBuf,
    process::{self, Command},
    thread,
    time::Duration,
};

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
    let ap = Command::new(&middle_adapter_exe)
        .current_dir(&workspace_dir)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .expect("middle-adapter.exe spawn");

    let mut ap_stdin = ap.stdin.unwrap();

    eprintln!("write initialize");
    let mut seq = 0;
    let initialize_req = {
        seq += 1;
        format!(
            r#"{{"type": "request", "command": "initialize", "seq": {}}}"#,
            seq
        )
    };
    write!(
        ap_stdin,
        "Content-Length: {}\n\n{}",
        initialize_req.len(),
        &initialize_req
    )
    .unwrap();

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
    write!(
        ap_stdin,
        "Content-Length: {}\n\n{}",
        launch_req.len(),
        &launch_req
    )
    .unwrap();

    let _disconnect = Defer::new(move || {
        eprintln!("write disconnect");
        let disconnect_req = {
            seq += 1;
            format!(
                r#"{{"type": "request", "command": "disconnect", "seq": {}}}"#,
                seq
            )
        };
        write!(
            ap_stdin,
            "Content-Length: {}\n\n{}",
            disconnect_req.len(),
            &disconnect_req
        )
        .unwrap();
    });

    eprintln!("sleep for a while");
    thread::sleep(Duration::from_millis(30 * 1000));
}

struct Defer<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Defer<F> {
    fn new(f: F) -> Self {
        Defer(Some(f))
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        eprintln!("drop");
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
