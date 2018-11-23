use hspsdk;

pub(crate) trait HspDebug {
    /// デバッグモードを変更する。中断やステップインなどの状態にできる。
    fn set_mode(&mut self, mode: hspsdk::DebugMode);

    /// グローバル変数のリストを送信する。
    fn get_globals(&self, seq: i64);

    fn terminate(&self);
}
