//! VSCode 側のデバッガーアダプターと通信する。

#![allow(unused_imports)]

use helpers::failwith;
use logger;
use std;
use std::sync::mpsc::{channel, Receiver as ChannelReceiver, Sender as ChannelSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;
use ws;

pub enum Response {
    StopOnBreakpoint,
}

pub struct WsClient {
    out: Arc<Mutex<ws::Sender>>,
    join_handle: std::thread::JoinHandle<()>,
}

pub struct Connection {
    sender: ws::Sender,
    pub join_handle: std::thread::JoinHandle<()>,
}

static mut CONNECTION: Option<std::sync::Mutex<Option<Connection>>> = None;

pub fn init_mod() {
    unsafe {
        CONNECTION = Some(std::sync::Mutex::new(None));
    }
}

pub fn with_connection<R, F>(f: F) -> R
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
    pub fn spawn() {
        let (sender, receiver) = channel::<ws::Message>();
        let (out_sender, out_receiver) = channel();

        let join_handle = std::thread::spawn(move || {
            // VSCode のデバッグセッションが開始したときに実行されるデバッグアダプターが WebSocket サーバーを立てているはずなので、それに接続する。
            ws::connect("ws://localhost:8089/", |out: ws::Sender| {
                // メッセージを送信するためのオブジェクトをスレッドの外部に送る。
                out_sender.send(out).unwrap();

                // メッセージを受信したときは、単にメッセージを外部に転送する。
                |message: ws::Message| {
                    sender.send(message).unwrap();
                    Ok(())
                }
            })
            .unwrap_or_else(|e| failwith(e))
        });

        let out = out_receiver.recv().unwrap();

        let connection = Connection {
            sender: out,
            join_handle,
        };

        unsafe {
            let mutex: &mut _ = CONNECTION.as_mut().unwrap();
            let lock = mutex.lock();
            *(lock.unwrap()) = Some(connection);
        }

        std::thread::sleep(std::time::Duration::from_secs(3));
        with_connection(|c| {
            c.sender
                .send(r#"{"type":"stopOnBreakpoint","line":6}"#)
                .unwrap();
        });
    }
}
