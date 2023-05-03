use log::{debug, error};
use std::io::{BufRead, Write};

pub struct DebugAdapterReader<R> {
    r: R,
    line: String,
}

impl<R: BufRead> DebugAdapterReader<R> {
    pub fn new(r: R) -> Self {
        DebugAdapterReader {
            r,
            line: String::new(),
        }
    }

    pub fn into_inner(self) -> R {
        self.r
    }

    pub fn recv(&mut self, body: &mut Vec<u8>) -> bool {
        // ヘッダー行を読む。
        self.line.clear();
        self.r.read_line(&mut self.line).unwrap();
        if self.line.is_empty() {
            // EOF.
            return false;
        }
        debug!("[DAC] recv (line: {:?})", self.line);

        if !self.line.starts_with("Content-Length:") {
            error!("ERROR expected content-length header");
            panic!()
        }

        let len = match self.line["Content-Length:".len()..].trim().parse::<usize>() {
            Err(err) => {
                error!("ERROR Expected content-length {:?}", err);
                panic!()
            }
            Ok(len) => len,
        };
        debug!("[DAC]   \\- (len: {:?})", len);

        // ヘッダーの終わりの `\r\n` を読み飛ばす。
        self.line.clear();
        self.r.read_line(&mut self.line).unwrap();

        // 本体を読む。
        body.resize(len, 0);
        self.r.read_exact(body).unwrap();

        debug!("受信 {}", String::from_utf8_lossy(body));

        true
    }
}

pub struct DebugAdapterWriter<W> {
    w: W,
    buffer: Vec<u8>,
}

impl<W: Write> DebugAdapterWriter<W> {
    pub fn new(w: W) -> Self {
        DebugAdapterWriter {
            w,
            buffer: Vec::new(),
        }
    }

    pub fn with_buffer(w: W, buffer: Vec<u8>) -> Self {
        DebugAdapterWriter { w, buffer }
    }

    pub fn into_inner(self) -> (W, Vec<u8>) {
        (self.w, self.buffer)
    }

    pub fn write<T: serde::Serialize>(&mut self, obj: &T) {
        debug!(
            "送信 {}",
            serde_json::to_string(obj).unwrap_or(String::new())
        );

        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, obj).unwrap();

        write!(&mut self.w, "Content-Length: {}\r\n\r\n", self.buffer.len()).unwrap();
        self.w.write_all(&self.buffer).unwrap();
        self.w.flush().unwrap();
    }
}
