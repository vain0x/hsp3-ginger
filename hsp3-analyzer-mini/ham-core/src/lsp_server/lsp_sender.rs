use super::{LspError, LspErrorResponse, LspNotification, LspRequest, LspResponse, Outgoing};
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

    pub(crate) fn send<R: serde::Serialize>(&mut self, outgoing: Outgoing<R>) {
        match outgoing {
            Outgoing::Request { id, method, params } => {
                self.send_request(id, method, params)
            }
            Outgoing::Notification { method, params } => {
                self.send_notification(method, params)
            }
            Outgoing::Response { id, result } => self.send_response(id, result),
            Outgoing::Error {
                id,
                code,
                msg,
                data,
            } => self.send_error_code(id, code, msg, data),
        }
    }

    fn send_request<P: serde::Serialize>(&mut self, id: Value, method: String, params: P) {
        let mut buf = Vec::new();
        serde_json::to_writer(
            &mut buf,
            &LspRequest::<P> {
                jsonrpc: "2.0".to_string(),
                id,
                method,
                params,
            },
        )
        .unwrap();

        self.do_send(&buf);
    }

    fn send_notification<P: serde::Serialize>(&mut self, method: String, params: P) {
        let mut buf = Vec::new();
        serde_json::to_writer(
            &mut buf,
            &LspNotification::<P> {
                jsonrpc: "2.0".to_string(),
                method,
                params,
            },
        )
        .unwrap();

        self.do_send(&buf);
    }

    fn send_response<R: serde::Serialize>(&mut self, id: Value, result: R) {
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

    fn send_error_code<R: serde::Serialize>(
        &mut self,
        id: Option<Value>,
        code: i64,
        msg: String,
        data: R,
    ) {
        let mut buf = Vec::new();

        serde_json::to_writer(
            &mut buf,
            &LspErrorResponse {
                jsonrpc: "2.0".to_string(),
                id: id.unwrap_or(Value::Null),
                error: LspError { code, msg, data },
            },
        )
        .unwrap();

        self.do_send(&buf);
    }
}
