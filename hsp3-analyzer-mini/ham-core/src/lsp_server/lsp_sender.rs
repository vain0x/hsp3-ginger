use super::{LspError, LspErrorResponse, LspNotification, LspRequest, LspResponse};
use serde_json::Value;
use std::io::{self, Write as _};

pub(super) struct LspSender<W: io::Write> {
    out: io::BufWriter<W>,
}

impl<W: io::Write> LspSender<W> {
    pub(crate) fn new(out: W) -> LspSender<W> {
        LspSender {
            out: io::BufWriter::new(out),
        }
    }

    fn do_send(&mut self, content: &[u8]) {
        let content_length = content.len();
        let content = String::from_utf8_lossy(content);

        write!(
            self.out,
            "Content-Length: {}\r\n\r\n{}",
            content_length, content
        )
        .unwrap();
        self.out.flush().unwrap();

        trace!(
            "lsp-sender/send Content-Length: {}\r\n\r\n{}",
            content_length,
            if content_length < 0x1000 {
                &content
            } else {
                "TOO_LONG"
            }
        );
    }

    pub(crate) fn send_request<P: serde::Serialize>(&mut self, id: i64, method: &str, params: P) {
        let mut buf = Vec::new();
        serde_json::to_writer(
            &mut buf,
            &LspRequest::<P> {
                jsonrpc: "2.0".to_string(),
                id,
                method: method.to_string(),
                params,
            },
        )
        .unwrap();

        self.do_send(&buf);
    }

    pub(crate) fn send_notification<P: serde::Serialize>(&mut self, method: &str, params: P) {
        let mut buf = Vec::new();
        serde_json::to_writer(
            &mut buf,
            &LspNotification::<P> {
                jsonrpc: "2.0".to_string(),
                method: method.to_string(),
                params,
            },
        )
        .unwrap();

        self.do_send(&buf);
    }

    pub(crate) fn send_response<R: serde::Serialize>(&mut self, id: i64, result: R) {
        let mut buf = Vec::new();
        serde_json::to_writer(
            &mut buf,
            &LspResponse::<R> {
                jsonrpc: "2.0".to_string(),
                id,
                result,
            },
        )
        .unwrap();

        self.do_send(&buf);
    }

    pub(crate) fn send_error_code(&mut self, id: Option<Value>, code: i64, msg: &str) {
        let mut buf = Vec::new();

        serde_json::to_writer(
            &mut buf,
            &LspErrorResponse {
                jsonrpc: "2.0".to_string(),
                id: id.unwrap_or(Value::Null),
                error: LspError {
                    code,
                    msg: msg.to_string(),
                    // data: Value::Null,
                },
            },
        )
        .unwrap();

        self.do_send(&buf);
    }
}
