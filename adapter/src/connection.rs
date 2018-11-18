//! VSCode 側のデバッガーアダプターと通信する。

#![allow(unused_imports)]

use helpers::failwith;
use hsprt;
use hspsdk;
use logger;
use std;
use std::sync::mpsc::{self, channel, Receiver as ChannelReceiver, Sender as ChannelSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;
use ws;

pub(crate) enum Response {
    StopOnBreakpoint,
}

#[derive(Clone, Debug)]
pub(crate) enum Request {
    FromEditor(String),
    FromSelf(i32),
}

pub(crate) struct Connection {
    sender: ws::Sender,
    pub req_sender: mpsc::Sender<Request>,
    pub join_handle: std::thread::JoinHandle<()>,
}

static mut CONNECTION: Option<std::sync::Mutex<Option<Connection>>> = None;

pub(crate) fn init_mod() {
    unsafe {
        CONNECTION = Some(std::sync::Mutex::new(None));
    }
}

pub(crate) fn with_connection<R, F>(f: F) -> R
where
    F: Fn(&mut Connection) -> R,
{
    unsafe {
        let mutex: &mut _ = CONNECTION.as_mut().unwrap();
        let mut lock = mutex.lock().unwrap();
        f((*lock).as_mut().unwrap())
    }
}

impl Connection {
    pub fn spawn<D>(mut d: D)
    where
        D: hsprt::HspDebug + Send + 'static,
    {
        let (sender, receiver) = channel::<Request>();
        let (out_sender, out_receiver) = channel();
        let req_sender = sender.clone();

        let join_handle = std::thread::spawn(move || {
            // VSCode のデバッグセッションが開始したときに実行されるデバッグアダプターが WebSocket サーバーを立てているはずなので、それに接続する。
            ws::connect("ws://localhost:8089/", |out: ws::Sender| {
                // メッセージを送信するためのオブジェクトをスレッドの外部に送る。
                out_sender.send(out).unwrap();

                // メッセージを受信したときは、単にメッセージを外部に転送する。
                |message: ws::Message| {
                    match message {
                        ws::Message::Binary(_) => {
                            logger::log("受信 失敗 バイナリ");
                        }
                        ws::Message::Text(json) => {
                            sender.send(Request::FromEditor(json)).unwrap();
                        }
                    }
                    Ok(())
                }
            }).unwrap_or_else(|e| failwith(e))
        });

        let out = out_receiver.recv().unwrap();

        let j = std::thread::spawn(move || {
            while let Ok(message) = receiver.recv() {
                match message {
                    Request::FromEditor(message) => {
                        logger::log(&format!("受信 FromEditor({})", &message));
                        if message.contains("continue") {
                            d.set_mode(hspsdk::HSPDEBUG_RUN as hspsdk::DebugMode);
                            with_connection(|c| {
                                c.sender.send(r#"{"type":"continue"}"#).unwrap();
                            });
                        } else if message.contains("pause") {
                            d.set_mode(hspsdk::HSPDEBUG_STOP as hspsdk::DebugMode);
                            with_connection(|c| {
                                c.sender
                                    .send(r#"{"type":"stopOnBreakpoint","line":6}"#)
                                    .unwrap();
                            });
                        } else if message.contains("next") {
                            d.set_mode(hspsdk::HSPDEBUG_STEPIN as hspsdk::DebugMode);
                        } else {
                            logger::log("  不明なメッセージ");
                        }
                    }
                    Request::FromSelf(line) => {
                        logger::log("送信 break");
                        with_connection(|c| {
                            c.sender
                                .send(format!(r#"{{"type":"stopOnBreakpoint","line":{} }}"#, line))
                                .unwrap();
                        })
                    }
                }
            }
        });

        let connection = Connection {
            sender: out,
            req_sender: req_sender,
            join_handle,
        };

        unsafe {
            let mutex: &mut _ = CONNECTION.as_mut().unwrap();
            let lock = mutex.lock();
            *(lock.unwrap()) = Some(connection);
        }
    }

    /// VSCode 側にリクエストを送る。
    pub fn send_request(&self, request: Request) -> Option<()> {
        logger::log(&format!("Request {:?}", request));
        self.req_sender
            .send(request)
            .map_err(|e| logger::log_error(&e))
            .ok()
    }
}
