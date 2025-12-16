use colored::Colorize;
use std::fmt::Display;
use std::io::Write;
use std::sync::{LazyLock, Mutex};
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

pub struct Log {
    sinks: Vec<Box<dyn Write>>,
    buffer: String,
}

static LOG: LazyLock<Mutex<Log>> = LazyLock::new(|| Mutex::new(Log::new()));

unsafe impl std::marker::Send for Log {}

impl Log {
    fn new() -> Self {
        let l = Log {
            sinks: vec![],
            buffer: String::new(),
        };
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_secs(1));
                let sinks = &mut LOG.lock().unwrap().sinks;
                let mut lock = LOG.lock().unwrap();
                for sink in sinks {
                    write!(sink, "{}", lock.buffer).unwrap();
                }
                lock.buffer.clear();
            }
        });
        l
    }

    pub fn add(elem: Box<dyn Write>) {
        LOG.lock().unwrap().sinks.push(elem)
    }

    pub fn write(priority: Priority, output: &str) {
        if LOG.lock().unwrap().sinks.is_empty() {
            return;
        }
        LOG.lock()
            .unwrap()
            .buffer
            .push_str(&format!("{}: {}\n", priority, output));
    }
}

#[cfg(test)]
mod tests {}
