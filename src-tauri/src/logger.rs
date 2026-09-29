use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use chrono::Utc;

const LOG_FILE_NAME: &str = "gym.log";
const MAX_LOG_BYTES: u64 = 512 * 1024;
const RETAINED_LOG_FILES: usize = 3;

static LOGGER: OnceLock<Mutex<RotatingLog>> = OnceLock::new();

pub fn init(app_data_directory: &Path) -> io::Result<PathBuf> {
    let log_directory = app_data_directory.join("logs");
    fs::create_dir_all(&log_directory)?;
    let log_path = log_directory.join(LOG_FILE_NAME);
    let rotating_log = RotatingLog::new(log_path.clone(), MAX_LOG_BYTES, RETAINED_LOG_FILES);
    let _ = LOGGER.set(Mutex::new(rotating_log));
    info("Gym process started");
    Ok(log_path)
}

pub fn info(message: impl AsRef<str>) {
    write("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write("ERROR", message.as_ref());
}

fn write(level: &str, message: &str) {
    let line = format!("{} {level} {message}\n", Utc::now().to_rfc3339());
    eprint!("{line}");
    if let Some(logger) = LOGGER.get() {
        match logger.lock() {
            Ok(mut logger) => {
                if let Err(error) = logger.write_line(line.as_bytes()) {
                    eprintln!("could not write local log: {error}");
                }
            }
            Err(_) => eprintln!("local logger state is unavailable"),
        }
    }
}

struct RotatingLog {
    path: PathBuf,
    max_bytes: u64,
    retained_files: usize,
}

impl RotatingLog {
    fn new(path: PathBuf, max_bytes: u64, retained_files: usize) -> Self {
        Self {
            path,
            max_bytes,
            retained_files,
        }
    }

    fn write_line(&mut self, line: &[u8]) -> io::Result<()> {
        let current_size = fs::metadata(&self.path).map_or(0, |metadata| metadata.len());
        if current_size.saturating_add(line.len() as u64) > self.max_bytes {
            self.rotate()?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line)
    }

    fn rotate(&self) -> io::Result<()> {
        if self.retained_files == 0 {
            return File::create(&self.path).map(|_| ());
        }
        for index in (1..self.retained_files).rev() {
            let source = numbered_path(&self.path, index);
            let destination = numbered_path(&self.path, index + 1);
            if source.exists() {
                fs::rename(source, destination)?;
            }
        }
        if self.path.exists() {
            fs::rename(&self.path, numbered_path(&self.path, 1))?;
        }
        Ok(())
    }
}

fn numbered_path(path: &Path, index: usize) -> PathBuf {
    path.with_file_name(format!("{LOG_FILE_NAME}.{index}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_and_limits_the_number_of_log_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOG_FILE_NAME);
        let mut log = RotatingLog::new(path.clone(), 12, 2);

        for index in 0..6 {
            log.write_line(format!("line-{index}\n").as_bytes())
                .unwrap();
        }

        assert!(path.exists());
        assert!(numbered_path(&path, 1).exists());
        assert!(numbered_path(&path, 2).exists());
        assert!(!numbered_path(&path, 3).exists());
    }
}
