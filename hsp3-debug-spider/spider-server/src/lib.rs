#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate rouille;

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

type SetModeFn = extern "C" fn(i32);

type LogFn = extern "C" fn(*const u16, usize);

struct DebugLogger;

lazy_static! {
    static ref DEBUG_LOG_FN: Mutex<Option<LogFn>> = Mutex::default();
}

impl DebugLogger {
    fn init(log_fn: LogFn) {
        let mut lock = DEBUG_LOG_FN.lock().unwrap();
        *lock = Some(log_fn);

        log::set_max_level(log::LevelFilter::Trace);
        log::set_logger(&DebugLogger).expect("set_logger");
    }
}

impl log::Log for DebugLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let text = format!("{} - {}\n", record.level(), record.args());
        let text = text.encode_utf16().collect::<Vec<u16>>();

        let log_fn = match DEBUG_LOG_FN.lock() {
            Err(_) => return,
            Ok(lock) => match lock.as_ref() {
                None => return,
                Some(&log_fn) => log_fn,
            },
        };

        log_fn(text.as_ptr(), text.len());
    }

    fn flush(&self) {
        // pass
    }
}

struct Global {
    logmes: String,

    set_mode: SetModeFn,

    #[allow(unused)]
    join_handle: JoinHandle<()>,

    child: std::process::Child,
}

lazy_static! {
    static ref GLOBAL: Mutex<Option<Global>> = Mutex::new(None);
}

fn with_global(f: impl FnOnce(&mut Global)) {
    let mut lock = match GLOBAL.lock() {
        Err(err) => {
            warn!("can't lock global {:?}", err);
            return;
        }
        Ok(lock) => lock,
    };

    let global = match lock.as_mut() {
        None => {
            warn!("before initialization");
            return;
        }
        Some(global) => global,
    };

    f(global);
}

fn start_server() {
    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist");
    trace!("dist_dir = {:?}", dist_dir);

    rouille::start_server_with_pool("localhost:8080", None, move |request| {
        // Serve dist
        {
            let response = rouille::match_assets(&request, &dist_dir);
            if response.is_success() {
                return response;
            }
        }

        // API
        router! (request,
            (GET) (/logmes) => {
                let mut res = None;
                with_global(|global| {
                    res = Some(rouille::Response::text(&global.logmes));
                });
                res.unwrap()
            },
            (POST) (/continue) => {
                let mut set_mode = None;
                with_global(|global| {
                    set_mode = Some(global.set_mode);
                });
                trace!("continue");
                (set_mode.unwrap())(1);
                rouille::Response::text("")
            },
            _ => rouille::Response::html("404").with_status_code(404)
        )
    });
}

#[no_mangle]
extern "C" fn spider_server_initialize(set_mode: SetModeFn, log_fn: LogFn) {
    // info! などのログ出力が log_fn 関数を使うように設定する。
    DebugLogger::init(log_fn);

    trace!("spider_server_initialize");

    let mut lock = GLOBAL.lock().ok().expect("lock global");
    if lock.is_some() {
        panic!("already initialized");
    }

    let join_handle = thread::spawn(|| start_server());

    let browser_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../spider-browser/SpiderBrowser/bin/x64/Release/SpiderBrowser.exe");
    let child = std::process::Command::new(browser_path)
        .spawn()
        .expect("spawn browser");

    *lock = Some(Global {
        logmes: String::new(),
        set_mode,
        child,
        join_handle,
    });
}

#[no_mangle]
extern "C" fn spider_server_terminate() {
    let mut lock = GLOBAL.lock().unwrap();

    // NOTE: 本来はここでサーバーに停止命令を送ってからサーバースレッドに join し、
    //       サーバーを安全に停止させるべきですが、rouille のサーバーを停止させる方法を
    //       まだ調べていません。サーバースレッドはプロセス終了時に abort します。
    let global = lock.take();

    if let Some(mut global) = global {
        global.child.kill().ok();
    }
}

#[no_mangle]
extern "C" fn spider_server_logmes(data: *const u8, size: usize) {
    let text = unsafe { std::slice::from_raw_parts(data, size) };
    trace!("logmes '{}'", String::from_utf8_lossy(text).as_ref());

    with_global(|global| {
        // FIXME: 文字コード
        global.logmes += String::from_utf8_lossy(text).as_ref();
        global.logmes += "\r\n";
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
