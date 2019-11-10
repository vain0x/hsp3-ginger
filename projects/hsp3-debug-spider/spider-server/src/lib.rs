#[macro_use]
extern crate lazy_static;

use std::cell::RefCell;
use std::io::Write;
use std::net;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

struct Global {
    #[allow(unused)]
    join_handle: JoinHandle<()>,
}

lazy_static! {
    static ref GLOBAL: Mutex<RefCell<Option<Global>>> = Mutex::new(RefCell::new(None));
}

#[no_mangle]
extern "C" fn spider_server_initialize() {
    let lock = GLOBAL.lock().unwrap();
    let mut cell = lock.borrow_mut();
    if cell.is_some() {
        panic!("already initialized");
    }

    let join_handle = thread::spawn(move || {
        let listener = match net::TcpListener::bind(("0.0.0.0", 8080)) {
            Err(err) => panic!("can't start server {:?}", err),
            Ok(stream) => stream,
        };

        for income in listener.incoming() {
            let mut stream = match income {
                Err(err) => panic!("bad incoming {:?}", err),
                Ok(stream) => stream,
            };

            let body = "<html><head><title>Hello world!</title></head><body><h1>HELLO WORLD</h1></body></html>";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; utf-8\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });

    *cell = Some(Global { join_handle });
}

#[no_mangle]
extern "C" fn spider_server_terminate() {
    let lock = GLOBAL.lock().unwrap();
    let mut cell = lock.borrow_mut();

    cell.take();
    // FIXME: スレッドに join する
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
