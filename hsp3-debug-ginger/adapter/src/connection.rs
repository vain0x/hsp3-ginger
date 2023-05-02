//! デバッグアダプターを通じて開発ツール (VSCode) と通信する

use crate::app;
use log::{debug, error, info};
use shared::{debug_adapter_connection as dac, debug_adapter_protocol as dap};
use std::{
    fs::{self, File},
    io::BufReader,
    sync::mpsc,
};

/// コネクションワーカーが扱える操作。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Send(dap::Msg),
}

/// コネクションワーカーに処理を依頼するもの。
#[derive(Clone, Debug)]
pub(crate) struct Sender {
    sender: mpsc::SyncSender<Action>,
}

impl Sender {
    pub fn send(&self, action: Action) {
        self.sender
            .send(action)
            .unwrap_or_else(|err| error!("[connection::Sender] {:?}", err));
    }
}

/// コネクションワーカー
///
/// デバッグアダプター (middle-adapter) が生成した名前付きパイプのクライアント側の端を開き、
/// それを使ってメッセージの読み書きを行う
pub(crate) struct Worker {
    rx: mpsc::Receiver<Action>,
    finish_tx: mpsc::SyncSender<()>,

    app_sender: app::Sender,
    /// readerスレッドにストリームを渡すためのチャネル
    stream_tx: mpsc::SyncSender<File>,
}

impl Worker {
    pub fn new(
        app_sender: app::Sender,
    ) -> (Self, mpsc::Receiver<()>, Sender, Reader, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::sync_channel::<Action>(8);
        let sender = Sender { sender: tx };

        let (stream_tx, stream_rx) = mpsc::sync_channel(0);
        let (worker_finish_tx, worker_finish_rx) = mpsc::sync_channel(1);
        let (reader_finish_tx, reader_finish_rx) = mpsc::sync_channel(1);

        let worker = Worker {
            rx,
            finish_tx: worker_finish_tx,
            app_sender: app_sender.clone(),
            stream_tx,
        };

        let reader = Reader {
            stream_rx,
            finish_tx: reader_finish_tx,
            app_sender,
        };

        (worker, worker_finish_rx, sender, reader, reader_finish_rx)
    }

    pub fn run(self) {
        // middle-adpterが生成した名前付きパイプを開く
        // (クライアント側は普通にファイルとして開くことができる)
        // また、パイプを読み込み用と書き込み用に複製する
        // (同一のパイプを指すオブジェクトを2つ作るということ。
        //  Rustの所有権ルールのため、2つのスレッドからパイプにアクセスするためにはパイプへの参照が2つ必要となる)

        info!("[connection] 接続中");
        let in_stream = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(r"\\.\pipe\hdg-pipe")
            .expect("Open pipe");

        let mut out_stream = in_stream.try_clone().expect("Duplicate pipe");

        debug!("[connection] Send stream");
        self.stream_tx.send(in_stream).unwrap();

        info!("[connection] 開始");
        self.app_sender.send(app::Action::AfterConnected);

        loop {
            let msg = match self.rx.recv() {
                Ok(msg) => msg,
                Err(err) => {
                    error!("[connection::Worker] {:?}", err);
                    break;
                }
            };
            match msg {
                Action::Send(msg) => {
                    debug!("[connection::Worker] Send");
                    dac::DebugAdapterWriter::new(&mut out_stream).write(&msg);
                }
            }
        }

        info!("[connection] 終了");
        self.finish_tx.send(()).unwrap_or_else(|_| {
            error!("[connection] finish_tx.send");
        });
    }
}

/// パイプを監視してメッセージを読み取るためのワーカー
pub(crate) struct Reader {
    stream_rx: mpsc::Receiver<File>,
    finish_tx: mpsc::SyncSender<()>,
    app_sender: app::Sender,
}

impl Reader {
    pub fn run(self) {
        debug!("[reader] 接続待ち");
        let mut in_stream = self.stream_rx.recv().unwrap();
        debug!("[reader] received in_stream");

        let mut r = dac::DebugAdapterReader::new(BufReader::new(&mut in_stream));
        let mut buf = Vec::with_capacity(4096);
        loop {
            debug!("[reader] 受信待ち");
            if !r.recv(&mut buf) {
                break;
            }

            let msg = match serde_json::from_slice::<dap::Msg>(&buf) {
                Err(err) => {
                    error!("[reader] {:?}", err);
                    continue;
                }
                Ok(msg) => msg,
            };

            debug!("[reader] 受信");
            self.app_sender.send(app::Action::AfterRequestReceived(msg));
        }

        debug!("[reader] 終了");
        self.finish_tx.send(()).unwrap_or_else(|_| {
            error!("[reader] finish_tx.send");
        });
    }
}
