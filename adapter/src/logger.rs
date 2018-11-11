use std;
use std::path::PathBuf;

struct FileLogger {
    content: String,
    file_path: PathBuf,
}

static mut LOGGER: Option<std::sync::Mutex<Option<FileLogger>>> = None;

pub fn init_mod() {
    unsafe {
        LOGGER = Some(std::sync::Mutex::new(None));
    }
}

fn with_logger<R, F>(f: F) -> R
where
    F: Fn(&mut FileLogger) -> R,
{
    unsafe {
        let logger_mutex: &mut _ = LOGGER.as_mut().unwrap();
        let mut logger_lock = logger_mutex.lock().unwrap();

        // Initialize.
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
