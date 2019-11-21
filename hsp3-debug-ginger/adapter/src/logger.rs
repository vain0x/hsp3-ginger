use crate::log;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync;

#[derive(Clone, Copy, Debug)]
struct MyLogger;

impl MyLogger {
    fn init() {
        log::set_logger(&MyLogger).unwrap();
        log::set_max_level(log::LevelFilter::Debug);
    }
}

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            with_logger(|logger| {
                writeln!(logger.file, "{} {}", record.level(), record.args()).unwrap();
            });
        }
    }

    fn flush(&self) {
        with_logger(|logger| {
            logger.flush();
        });
    }
}

struct FileLogger {
    file: io::BufWriter<fs::File>,
}

impl FileLogger {
    fn create(file_path: &Path) -> io::Result<FileLogger> {
        let file = fs::File::create(file_path)?;
        let file = io::BufWriter::new(file);
        Ok(FileLogger { file })
    }

    fn flush(&mut self) {
        self.file.flush().ok();
    }
}

enum LazyInit<T> {
    /// 未初期化。
    Uninit,
    /// 破棄済み。
    Deinit,
    /// 初期化済み。
    Value(T),
}

static mut LOGGER: Option<sync::Mutex<LazyInit<FileLogger>>> = None;

/// モジュールの初期化処理を行う。
pub(crate) fn initialize_mod() {
    unsafe { LOGGER = Some(sync::Mutex::new(LazyInit::Uninit)) };

    MyLogger::init();
}

/// モジュールの終了時の処理を行う。
pub(crate) fn deinitialize_mod() {
    (|| {
        info!("[logger] 終了");

        let mutex = unsafe { LOGGER.as_ref() }?;
        let mut lock = mutex.lock().ok()?;
        if let LazyInit::Value(ref mut logger) = *lock {
            logger.flush();
        }
        *lock = LazyInit::Deinit;
        Some(())
    })();
}

/// ロガーを使った処理を行う。
fn with_logger<F>(f: F)
where
    F: Fn(&mut FileLogger),
{
    (|| {
        // NOTE: static mut 変数へのアクセスは unsafe 。
        let logger_mutex: &sync::Mutex<_> = unsafe { LOGGER.as_ref() }?;

        // ロガーの所有権を一時的に借用する。
        let mut logger_lock = logger_mutex.lock().ok()?;

        // 初めてロガーを使用するときのみ、初期化を行う。
        if let LazyInit::Uninit = *logger_lock {
            let logger = match FileLogger::create(&log_file_path()) {
                Ok(logger) => LazyInit::Value(logger),
                Err(_) => LazyInit::Deinit,
            };
            *logger_lock = logger;
        }

        match *logger_lock {
            LazyInit::Uninit => unreachable!(),
            LazyInit::Deinit => {}
            LazyInit::Value(ref mut l) => f(l),
        }

        Some(())
    })();
}

#[allow(deprecated)]
fn log_file_path() -> PathBuf {
    std::env::home_dir()
        .map(|d| d.join("hsp3debug-rust.log"))
        .unwrap()
}
