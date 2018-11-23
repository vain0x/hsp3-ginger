use std;
use std::path::PathBuf;
use std::sync::Mutex;

struct FileLogger {
    content: String,
    file_path: PathBuf,
}

static mut LOGGER: Option<Mutex<Option<FileLogger>>> = None;

/// モジュールの初期化処理を行う。
pub fn initialize_mod() {
    unsafe {
        LOGGER = Some(Mutex::new(None));
    }
}

/// ロガーを使った処理を行う。
fn with_logger<R, F>(f: F) -> R
where
    F: Fn(&mut FileLogger) -> R,
{
    unsafe {
        // NOTE: static mut 変数へのアクセスは unsafe 。
        let logger_mutex: &mut _ = LOGGER.as_mut().unwrap();

        // ロガーの所有権を一時的に借用する。
        let mut logger_lock = logger_mutex.lock().unwrap();

        // 初めてロガーを使用するときのみ、初期化を行う。
        if (*logger_lock).is_none() {
            let l = FileLogger {
                content: String::new(),
                file_path: log_file_path(),
            };
            *logger_lock = Some(l);
        }

        f((*logger_lock).as_mut().unwrap())
    }
}

#[allow(deprecated)]
fn log_file_path() -> PathBuf {
    std::env::home_dir()
        .map(|d| d.join("hsp3debug-rust.log"))
        .unwrap()
}

pub fn log(message: &str) {
    with_logger(move |logger| {
        // Append.
        {
            logger.content += &message;
            logger.content += "\r\n";
        }

        // Rewrite.
        std::fs::write(&logger.file_path, &logger.content).unwrap();
    })
}

pub fn log_error<E: std::fmt::Debug>(err: &E) {
    let message = format!("[ERROR] {:?}", err);
    log(&message)
}
