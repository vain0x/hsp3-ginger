use hspsdk;

pub(crate) trait HspDebug {
    fn set_mode(&mut self, mode: hspsdk::DebugMode);
}
