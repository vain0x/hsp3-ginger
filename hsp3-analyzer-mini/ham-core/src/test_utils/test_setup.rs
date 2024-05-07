#![cfg(test)]

/// テスト中に `debug!()` などのログが出力されるようにする
#[allow(unused)]
pub(crate) fn set_test_logger() {
    env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init()
        .ok();
}
