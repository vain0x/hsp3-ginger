use std::io;
use std::io::{BufRead as _, Read as _};

pub(super) struct LspReceiver<R> {
    content: Vec<u8>,
    line: String,
    reader: io::BufReader<R>,
}

impl<R: io::Read> LspReceiver<R> {
    pub(crate) fn read_next<F: FnMut(&str)>(&mut self, mut f: F) {
        self.line.clear();
        self.reader.read_line(&mut self.line).expect("Read header");
        if !self.line.starts_with("Content-Length:") {
            error!("Unknown header {}", self.line);
            return;
        }

        let l = "Content-Length:".len();
        let r = self.line.len();
        let content_length = self.line[l..r]
            .trim()
            .parse::<usize>()
            .expect("content length to be integer");

        self.line.clear();
        self.reader.read_line(&mut self.line).expect("Read header");
        if self.line.trim().len() != 0 {
            error!("Unknown header {}", self.line);
            return;
        }

        self.content.resize(content_length, 0);
        self.reader
            .read_exact(&mut self.content)
            .expect("read payload");

        let json = String::from_utf8_lossy(&self.content);

        debug!("Received {}\n", json);

        f(&json);
    }

    pub fn new(reader: R) -> Self {
        LspReceiver {
            content: vec![],
            line: String::new(),
            reader: io::BufReader::new(reader),
        }
    }
}
