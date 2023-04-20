use log::Log;
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
    sync::Mutex,
};

pub struct FileLogger {
    file_m: Mutex<File>,
}

impl FileLogger {
    pub fn new(file_path: &Path) -> io::Result<FileLogger> {
        let file = File::create(file_path)?;
        Ok(FileLogger {
            file_m: Mutex::new(file),
        })
    }
}

impl Log for FileLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        // if !self.enabled(record.metadata()) {
        //     return;
        // }

        let mut f = self.file_m.lock().unwrap();

        write!(
            &mut *f,
            "[{}:{} {}] - {}\n",
            record.file().unwrap_or(""),
            record.line().unwrap_or(0),
            record.level(),
            record.args()
        )
        .unwrap();
    }

    fn flush(&self) {
        self.file_m.lock().unwrap().flush().ok();
    }
}
