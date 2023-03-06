//! Logging utilities.

use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Saves all log records in a global deque.
///
/// Uses a `DebugView` to access it.
pub struct CursiveLogger {
    /// Log filter level for log messages from within cursive
    int_filter_level: log::LevelFilter,
    /// Log filter level for log messages from sources outside of cursive
    ext_filter_level: log::LevelFilter,
    /// Size of log queue
    log_size: usize
}

fn get_env_log_level(env_var_name: &str) -> Option<log::LevelFilter> {
    match std::env::var(env_var_name) {
        Ok(mut log_level_str) => {
            log_level_str.make_ascii_uppercase();
            match log_level_str {
                level if level == "TRACE" => Some(log::LevelFilter::Trace),
                level if level == "DEBUG" => Some(log::LevelFilter::Debug),
                level if level == "INFO" => Some(log::LevelFilter::Info),
                level if level == "WARN" => Some(log::LevelFilter::Warn),
                level if level == "ERROR" => Some(log::LevelFilter::Error),
                _ => None,
            }
        }
        Err(_) => None,
    }
}

impl CursiveLogger {
    /// Creates a new CursiveLogger with default log filter levels of log::LevelFilter::Trace
    /// If RUST_LOG is set, then both internal and external log levels are set to match
    /// If CURSIVE_LOG is set, then the internal log level is set to match
    /// Remember to call `init()` to install with `log` backend
    pub fn new() -> Self {
        CursiveLogger {
            int_filter_level: log::LevelFilter::Trace,
            ext_filter_level: log::LevelFilter::Trace,
            log_size: 1000,
        }
    }

    /// sets the internal log filter level
    pub fn with_int_filter_level(mut self, level: log::LevelFilter) -> Self {
        self.int_filter_level = level;
        self
    }

    /// sets the external log filter level
    pub fn with_ext_filter_level(mut self, level: log::LevelFilter) -> Self {
        self.ext_filter_level = level;
        self
    }

    /// sets log filter levels based on environment variables `RUST_LOG` and `CURSIVE_LOG`
    pub fn with_env(mut self) -> Self {
        if let Some(filter_level) = get_env_log_level("RUST_LOG") {
            self.int_filter_level = filter_level;
            self.ext_filter_level = filter_level;
        }
        if let Some(filter_level) = get_env_log_level("CURSIVE_LOG") {
            self.int_filter_level = filter_level;
        }
        self
    }

    /// sets the size of the log queue
    pub fn with_log_size(mut self, log_size: usize) -> Self {
        self.log_size = log_size;
        self
    }

    /// installs the logger with log
    /// calling twice will panic
    pub fn init(self) {
        reserve_logs(self.log_size);
        log::set_logger(Box::leak(Box::new(self))).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
    }
}

/// A log record.
pub struct Record {
    /// Log level used for this record
    pub level: log::Level,
    /// Time this message was logged
    pub time: time::OffsetDateTime,
    /// Message content
    pub message: String,
}

lazy_static! {
    /// Circular buffer for logs. Use it to implement [`DebugView`].
    ///
    /// [`DebugView`]: ../views/struct.DebugView.html
    pub static ref LOGS: Mutex<VecDeque<Record>> =
        Mutex::new(VecDeque::new());
}

/// Log a record in cursive's log queue.
pub fn log(record: &log::Record) {
    let mut logs = LOGS.lock().unwrap();
    // TODO: customize the format? Use colors? Save more info?
    if logs.len() == logs.capacity() {
        logs.pop_front();
    }
    logs.push_back(Record {
        level: record.level(),
        message: format!("{}", record.args()),
        time: time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc()),
    });
}

impl log::Log for CursiveLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if metadata.target().contains("cursive_core") {
            metadata.level() <= self.int_filter_level
        } else {
            metadata.level() <= self.ext_filter_level
        }
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            log(record);
        }
    }

    fn flush(&self) {}
}

/// Initialize the Cursive logger.
///
/// Make sure this is the only logger your are using.
///
/// Use a [`DebugView`](crate::views::DebugView) to see the logs, or use
/// [`Cursive::toggle_debug_console()`](crate::Cursive::toggle_debug_console()).
pub fn init() {
    // This will panic if `set_logger` was already called.
    CursiveLogger::new().init();
}

/// Return a logger that stores records in cursive's log queue.
///
/// These logs can then be read by a [`DebugView`](crate::views::DebugView).
///
/// An easier alternative might be to use [`init()`].
pub fn get_logger() -> CursiveLogger {
    CursiveLogger::new()
}

/// Adds `n` more entries to cursive's log queue.
///
/// Most of the time you don't need to use this directly.
///
/// You should call this if you're not using `init()` nor `get_logger()`.
pub fn reserve_logs(n: usize) {
    LOGS.lock().unwrap().reserve(n);
}
