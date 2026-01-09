use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref LOGGER: Mutex<Option<File>> = Mutex::new(None);
}

pub fn init() {
    let mut logger = LOGGER.lock().unwrap();
    if logger.is_none()
        && let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("ai_debug.log")
        {
            *logger = Some(file);
        }
}

pub fn log(message: &str) {
    if let Some(logger) = LOGGER.lock().unwrap().as_mut() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = writeln!(logger, "[{}] {}", timestamp, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_init() {
        init();
    }

    #[test]
    fn test_logger_log() {
        init();
        log("Test log message");
    }
}
