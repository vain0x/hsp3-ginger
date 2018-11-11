//! VSCode 側のデバッガーと接続する。

#![allow(unused_imports)]

use helpers::failwith;
use logger;
use std;
use std::sync::mpsc::{channel, Receiver as ChannelReceiver, Sender as ChannelSender};
use std::thread::{self, sleep};
use std::time::Duration;
use ws;

pub enum Response {
    StopOnBreakpoint,
}

pub struct Connection {
    sender: ChannelSender<Response>,
    receiver: ChannelReceiver<Response>,
    join_handle: std::thread::JoinHandle<()>,
}

static mut CONNECTION: Option<std::sync::Mutex<Option<Connection>>> = None;

pub fn init_mod() {
    unsafe {
        CONNECTION = Some(std::sync::Mutex::new(None));
    }
}

fn on_message(out: &ws::Sender, message: &ws::Message) -> Result<(), ws::Error> {
    match message.clone() {
        ws::Message::Text(message) => {
            logger::log(&message);
        }
        ws::Message::Binary(_) => {}
    }
    // out.send(message)
    out.send(r#"{"type":"stopOnBreakpoint"}"#)?;
    Ok(())
}

impl Connection {
    pub fn spawn() {
        let (sender, receiver) = channel::<Response>();

        let join_handle = std::thread::spawn(|| {
            // VSCode のデバッグセッションが開始したときに実行されるデバッグアダプターが WebSocket サーバーを立てているはずなので、それに接続する。
            // 注意: クロージャーは Fn トレイトでなければいけないので、可変な状態を持てない。状態を Mutex<_> で包んで外部に置くことで回避する。
            ws::connect("ws://localhost:8089/", |out: ws::Sender| {
                move |message: ws::Message| on_message(&out, &message)
            }).unwrap_or_else(|e| failwith(e));
        });

        let connection = Connection {
            sender,
            receiver,
            join_handle,
        };

        unsafe {
            let mutex: &mut _ = CONNECTION.as_mut().unwrap();
            let mut lock = mutex.lock();
            *(lock.unwrap()) = Some(connection);
        }
    }
}
