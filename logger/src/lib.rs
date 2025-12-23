use colored::Colorize;
use lazy_static::lazy_static;
use std::fmt::Display;
use std::io::Write;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub enum Priority {
    Info,
    Warn,
    Error,
}

impl Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Info => write!(f, "{}", "[INFO]".blue()),
            Self::Warn => write!(f, "{}", "[WARN]".yellow()),
            Self::Error => write!(f, "{}", "[ERROR]".red()),
        }
    }
}

lazy_static! {
    static ref LOG: Log = Log {
        sinks: Mutex::new(vec![]),
        buffer: Mutex::new(String::new())
    };
}

unsafe impl std::marker::Send for Log {}
unsafe impl std::marker::Sync for Log {}

pub struct Log {
    sinks: Mutex<Vec<Box<dyn Write>>>,
    buffer: Mutex<String>,
}

impl Log {
    pub fn add(elem: Box<dyn Write>) {
        if LOG.sinks.lock().unwrap().is_empty() {
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(50));
                    let mut buffer_lock = LOG.buffer.lock().unwrap();
                    for sink in LOG.sinks.lock().unwrap().iter_mut() {
                        let _ = sink.write_all(buffer_lock.as_bytes());
                        let _ = sink.flush();
                    }
                    buffer_lock.clear();
                }
            });
        }
        LOG.sinks.lock().unwrap().push(elem);
    }

    pub fn write(priority: Priority, output: &str) {
        LOG.buffer
            .lock()
            .unwrap()
            .push_str(&format!("{}: {}\n", priority, output));
    }
}

#[cfg(test)]
mod tests {}
