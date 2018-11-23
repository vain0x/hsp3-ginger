use std::io::{BufRead, Write};
use std::{fmt, fs, path, sync};

pub(crate) trait Logger: Clone {
    fn log(&self, args: fmt::Arguments);
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NullLogger;

impl Logger for NullLogger {
    fn log(&self, _: fmt::Arguments) {}
}

#[derive(Clone, Debug)]
pub(crate) struct FileLogger {
    log_file: Option<sync::Arc<sync::Mutex<fs::File>>>,
}

impl FileLogger {
    #[allow(unused)]
    pub fn create(path: &path::Path) -> Self {
        FileLogger {
            log_file: fs::File::create(path)
                .ok()
                .map(|f| sync::Arc::new(sync::Mutex::new(f))),
        }
    }
}

impl Logger for FileLogger {
    fn log(&self, args: fmt::Arguments) {
        if let Some(ref rc) = self.log_file {
            let mutex: &sync::Mutex<_> = &**rc;
            let mut lock: Option<_> = mutex.lock().ok();
            if let Some(mut file) = lock {
                file.write_fmt(format_args!("{:?}\n", args)).unwrap();
            }
        }
    }
}

pub(crate) struct DebugAdapterReader<R, L> {
    r: R,
    l: L,
    line: String,
}

impl<R: BufRead, L: Logger> DebugAdapterReader<R, L> {
    pub fn new(r: R, l: L) -> Self {
        DebugAdapterReader {
            r,
            l,
            line: String::new(),
        }
    }

    fn log(&self, args: fmt::Arguments) {
        self.l.log(args);
    }

    pub fn recv(&mut self, body: &mut Vec<u8>) -> bool {
        // ヘッダー行を読む。
        self.line.clear();
        self.r.read_line(&mut self.line).unwrap();
        if self.line.is_empty() {
            // EOF.
            return false;
        }

        if !self.line.starts_with("Content-Length:") {
            self.log(format_args!("ERROR expected content-length header"));
            panic!()
        }

        let len = match self.line["Content-Length:".len()..].trim().parse::<usize>() {
            Err(err) => {
                self.log(format_args!("ERROR Expected content-length {:?}", err));
                panic!()
            }
            Ok(len) => len,
        };

        // ヘッダーの終わりの `\r\n` を読み飛ばす。
        self.line.clear();
        self.r.read_line(&mut self.line).unwrap();

        // 本体を読む。
        body.resize(len, 0);
        self.r.read_exact(body).unwrap();

        {
            let args = format!("body={}", String::from_utf8_lossy(body));
            self.log(format_args!("{}", args));
        }

        true
    }
}

pub(crate) struct DebugAdapterWriter<W, L> {
    w: W,
    l: L,
    buffer: Vec<u8>,
}

impl<W: Write, L: Logger> DebugAdapterWriter<W, L> {
    pub fn new(w: W, l: L) -> Self {
        DebugAdapterWriter {
            w,
            l,
            buffer: Vec::new(),
        }
    }

    fn log(&self, args: fmt::Arguments) {
        self.l.log(args);
    }

    pub fn write<T: serde::Serialize>(&mut self, obj: &T) {
        self.log(format_args!(
            "送信 {}",
            serde_json::to_string(obj).unwrap()
        ));

        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, obj).unwrap();

        write!(&mut self.w, "Content-Length: {}\r\n\r\n", self.buffer.len()).unwrap();
        self.w.write_all(&self.buffer).unwrap();
        self.w.flush().unwrap();
    }
}
