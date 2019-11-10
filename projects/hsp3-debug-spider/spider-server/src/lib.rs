#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate rouille;

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

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

    #[allow(unused)]
    join_handle: JoinHandle<()>,
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

    rouille::start_server("localhost:8080", move |request| {
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
            _ => rouille::Response::html("404").with_status_code(404)
        )
    });
}

#[no_mangle]
extern "C" fn spider_server_initialize(log_fn: LogFn) {
    // info! などのログ出力が log_fn 関数を使うように設定する。
    DebugLogger::init(log_fn);

    trace!("spider_server_initialize");

    let mut lock = GLOBAL.lock().ok().expect("lock global");
    if lock.is_some() {
        panic!("already initialized");
    }

    let join_handle = thread::spawn(|| start_server());

    *lock = Some(Global {
        logmes: String::new(),
        join_handle,
    });
}

#[no_mangle]
extern "C" fn spider_server_terminate() {
    let mut lock = GLOBAL.lock().unwrap();

    lock.take();
    // FIXME: スレッドに join する
}

#[no_mangle]
extern "C" fn spider_server_logmes(data: *const u8, size: usize) {
    with_global(|global| {
        // FIXME: 文字コード
        let text = unsafe { std::slice::from_raw_parts(data, size) };

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
