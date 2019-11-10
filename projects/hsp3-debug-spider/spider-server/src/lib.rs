#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

use std::io::Write;
use std::net;
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
    #[allow(unused)]
    join_handle: JoinHandle<()>,
}

lazy_static! {
    static ref GLOBAL: Mutex<Option<Global>> = Mutex::new(None);
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

    let join_handle = thread::spawn(move || {
        let listener = match net::TcpListener::bind(("0.0.0.0", 8080)) {
            Err(err) => {
                warn!("can't start server {:?}", err);
                return;
            }
            Ok(stream) => stream,
        };

        trace!("listening...");
        for income in listener.incoming() {
            let mut stream = match income {
                Err(err) => {
                    warn!("bad incoming {:?}", err);
                    continue;
                }
                Ok(stream) => stream,
            };

            trace!("connected");
            let body = "<html><head><title>Hello world!</title></head><body><h1>HELLO WORLD</h1></body></html>";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; utf-8\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .and_then(|_| stream.write_all(body.as_bytes()))
            .and_then(|_| stream.flush())
            .map_err(|err| warn!("writing {:?}", err))
            .ok();
        }
    });

    *lock = Some(Global { join_handle });
}

#[no_mangle]
extern "C" fn spider_server_terminate() {
    let mut lock = GLOBAL.lock().unwrap();

    lock.take();
    // FIXME: スレッドに join する
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
