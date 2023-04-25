//! VSCode 側のデバッガーアダプターと通信する。

use crate::app;
use log::{debug, error, info, warn};
use named_pipe::PipeClient;
use shared::{debug_adapter_connection as dac, debug_adapter_protocol as dap};
use std::{
    fs::{self, File},
    io,
    sync::{mpsc, Arc, Mutex},
    thread,
};
use winapi::um::winbase::WaitNamedPipeA;

/// コネクションワーカーが扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Connect,
    Send(dap::Msg),
}

/// コネクションワーカーに処理を依頼するもの。
#[derive(Clone, Debug)]
pub(crate) struct Sender {
    sender: mpsc::Sender<Action>,
}

impl Sender {
    pub fn send(&self, action: Action) {
        self.sender
            .send(action)
            .map_err(|err| error!("[connection] {:?}", err))
            .ok();
    }
}

/// コネクションワーカー。
/// VSCode 側のデバッガーアダプターが立てている WebSocket サーバーに接続して双方向通信を行う。
pub(crate) struct Worker {
    app_sender: app::Sender,
    receiver: mpsc::Receiver<Action>,
    // connection: Option<(net::TcpStream, thread::JoinHandle<()>)>,
    // connection: Option<(File, thread::JoinHandle<()>)>,
    connection: Option<(PipeClient, thread::JoinHandle<()>)>,
}

impl Worker {
    pub fn new(app_sender: app::Sender) -> (Self, Sender) {
        let (sender, receiver) = mpsc::channel::<Action>();
        let sender = Sender { sender };

        let worker = Worker {
            app_sender,
            receiver,
            connection: None,
        };

        (worker, sender)
    }

    pub fn run(mut self) {
        loop {
            match self.receiver.recv() {
                Ok(Action::Connect) => {
                    // 接続要求が来たとき: 接続を試みる。
                    if self.connection.is_some() {
                        debug!("Already connected");
                        continue;
                    }

                    // let port = 57676;
                    // let stream = match net::TcpStream::connect(("127.0.0.1", port)) {
                    //     Ok(stream) => stream,
                    //     Err(err) => {
                    //         error!("{:?}", err);
                    //         continue;
                    //     }
                    // };
                    debug!("Connecting");
                    let pipe = PipeClient::connect(r"\\.\pipe\hdg-pipe").expect("connect");
                    debug!("connected");
                    let pipe = Arc::new(Mutex::new(pipe));
                    // let in_stream = PipeClient::connect(r"\\.\pipe\hdg-pipe").expect("connect(in)");
                    // debug!("connect(in)");
                    // let out_stream =
                    //     PipeClient::connect(r"\\.\pipe\hdg-pipe").expect("connect(out)");
                    // debug!("connect(out)");
                    // let in_stream = fs::OpenOptions::new()
                    //     .read(true)
                    //     .open(r"\\.\pipe\hdg-pipe")
                    //     .expect("open read pipe");
                    // let out_stream = fs::OpenOptions::new()
                    //     .write(true)
                    //     .append(true)
                    //     .open(r"\\.\pipe\hdg-pipe")
                    //     .expect("open write pipe");

                    // WaitNamedPipe
                    // debug!("Waiting pipes");
                    // {
                    //     let pipe_name = concat!(r"\\.\pipe\hdg-pipe", "\0");
                    //     let ok =
                    //         unsafe { WaitNamedPipeA(pipe_name.as_ptr() as *const i8, 1000u32) }
                    //             != 0;
                    //     debug!("wait={}", ok);
                    // }

                    // 受信したメッセージを処理するためのワーカースレッドを建てる。
                    let app_sender = self.app_sender.clone();
                    let s1 = pipe.clone();
                    let join_handle = thread::spawn(move || {
                        let in_stream = s1.lock().unwrap();
                        in_stream.read_async_owned(buf); // ?

                        let mut r =
                            dac::DebugAdapterReader::new(io::BufReader::new(&mut *in_stream));
                        let mut buf = Vec::new();
                        loop {
                            debug!("[dap-reader] recv");
                            if !r.recv(&mut buf) {
                                break;
                            }

                            let msg = match serde_json::from_slice::<dap::Msg>(&buf) {
                                Err(err) => {
                                    error!("[connection(2)] {:?}", err);
                                    continue;
                                }
                                Ok(msg) => msg,
                            };

                            app_sender.send(app::Action::AfterRequestReceived(msg));
                        }

                        info!("[connection] DAR 終了");
                    });

                    self.connection = Some((out_stream, join_handle));
                    self.app_sender.send(app::Action::AfterConnected);
                }
                Ok(Action::Send(msg)) => {
                    // 送信要求が来たとき: 接続が確立していたら送信する。
                    let mut stream = match self.connection {
                        None => {
                            warn!("送信 失敗 接続が確立していません");
                            continue;
                        }
                        Some((ref mut stream, _)) => stream,
                    };

                    dac::DebugAdapterWriter::new(&mut stream).write(&msg);
                }
                Err(err) => {
                    error!("[connection::Worker] {:?}", err);
                    break;
                }
            }
        }

        if let Some((stream, _)) = self.connection.take() {
            // stream.shutdown(net::Shutdown::Both).unwrap();
            drop(stream);

            // NOTE: なぜか停止しないので join しない。
            // join_handle.join().unwrap();
        }

        info!("[connection] 終了");
    }
}
