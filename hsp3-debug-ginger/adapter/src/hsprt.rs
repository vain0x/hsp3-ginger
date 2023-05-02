use crate::app;
use crate::hspsdk;
use log::error;
use std::sync::mpsc;

/// HSP ランタイムへの命令や問い合わせを表す。
#[derive(Clone, Debug)]
pub(crate) enum Action {
    /// デバッグモードを変更する。
    SetMode(hspsdk::DebugMode),
    /// 変数の中身を取得する。
    GetVar { seq: i64, var_path: app::VarPath },
    /// プログラムを終了する。
    Disconnect,
}

/// HSP ランタイム に処理を依頼するもの。
#[derive(Clone)]
pub(crate) struct Sender {
    sender: mpsc::SyncSender<Action>,
    notice_sender: mpsc::SyncSender<()>,
}

impl Sender {
    pub fn new(sender: mpsc::SyncSender<Action>, notice_sender: mpsc::SyncSender<()>) -> Self {
        Sender {
            sender,
            notice_sender,
        }
    }

    pub fn send(&self, action: Action, pausing: bool) {
        self.sender
            .send(action)
            .unwrap_or_else(|err| error!("[hsprt::Sender] {:?}", err));

        if pausing {
            self.notice_sender.send(()).ok();
        }
    }
}
