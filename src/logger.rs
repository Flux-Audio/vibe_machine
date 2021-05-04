use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Logger {
    log_file: Option<File>,
}

impl Logger {
    pub fn new(dir_path_str: &str, log_name: &str, enable: bool) -> Self {
        if enable {
            let dir_path = Path::new(dir_path_str);
            let _ = fs::create_dir_all(dir_path);
            let path = dir_path.join(log_name.to_owned() + ".log");

            let mut logger = Self {
                log_file: match File::create(&path) {
                    Err(_) => None,
                    Ok(file) => Some(file),
                },
            };

            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            logger.log(&format!("[instantiated on {:#?} ]\n", since_the_epoch)[..]);

            return logger;
        } else {
            Self {
                log_file: None,
            }
        }
    }

    pub fn log(&self, text: &str) {
        if !self.log_file.is_none() {
            let _ = self.log_file.as_ref().unwrap().write_all(text.as_bytes());
        }
    }
}