use hspsdk;
use logger;
use std::sync::mpsc;

/// HSP ランタイムへの命令や問い合わせを表す。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// デバッグモードを変更する。
    SetMode(hspsdk::DebugMode),
    /// グローバル変数を列挙する。
    GetGlobals { seq: i64 },
}

/// HSP ランタイム に処理を依頼するもの。
#[derive(Clone)]
pub(crate) struct Sender {
    sender: mpsc::Sender<Action>,
    notice_sender: mpsc::Sender<()>,
}

impl Sender {
    pub fn new(sender: mpsc::Sender<Action>, notice_sender: mpsc::Sender<()>) -> Self {
        Sender {
            sender,
            notice_sender,
        }
    }

    pub fn send(&self, action: Action, pausing: bool) {
        self.sender
            .send(action)
            .map_err(|err| logger::log_error(&err))
            .ok();

        if pausing {
            self.notice_sender.send(()).ok();
        }
    }
}
