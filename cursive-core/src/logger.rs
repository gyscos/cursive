//! Logging utilities.

use lazy_static::lazy_static;
use std::cmp::Ord;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::{Mutex, RwLock};

/// Saves all log records in a global deque.
///
/// Uses a `DebugView` to access it.
///
/// # Examples
///
/// Set log levels from env vars
///
/// ```
/// # use cursive_core::*;
/// logger::set_filter_levels_from_env();
/// logger::init();
/// ```
///
/// Set log levels explicitly.
///
/// ```
/// # use cursive_core::*;
/// # use log::LevelFilter;
/// logger::set_internal_filter_level(LevelFilter::Warn);
/// logger::set_external_filter_level(LevelFilter::Debug);
/// logger::init();
/// ```

pub struct CursiveLogger;

lazy_static! {
    /// Circular buffer for logs. Use it to implement [`DebugView`].
    ///
    /// [`DebugView`]: ../views/struct.DebugView.html
    pub static ref LOGS: Mutex<VecDeque<Record>> =
        Mutex::new(VecDeque::with_capacity(1_000));

    // Log filter level for log messages from within cursive
    static ref INT_FILTER_LEVEL: RwLock<log::LevelFilter> = RwLock::new(log::LevelFilter::Trace);
    // Log filter level for log messages from sources outside of cursive
    static ref EXT_FILTER_LEVEL: RwLock<log::LevelFilter> = RwLock::new(log::LevelFilter::Trace);
}

/// Sets the internal log filter level.
pub fn set_internal_filter_level(level: log::LevelFilter) {
    *INT_FILTER_LEVEL.write().unwrap() = level;
}

/// Sets the external log filter level.
pub fn set_external_filter_level(level: log::LevelFilter) {
    *EXT_FILTER_LEVEL.write().unwrap() = level;
}

/// Sets log filter levels based on environment variables `RUST_LOG` and `CURSIVE_LOG`.
/// If `RUST_LOG` is set, then both internal and external log levels are set to match.
/// If `CURSIVE_LOG` is set, then the internal log level is set to match with precedence over
/// `RUST_LOG`.
pub fn set_filter_levels_from_env() {
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        match log::LevelFilter::from_str(&rust_log) {
            Ok(filter_level) => {
                set_internal_filter_level(filter_level);
                set_external_filter_level(filter_level);
            }
            Err(e) => log::warn!("Could not parse RUST_LOG: {}", e),
        }
    }
    if let Ok(cursive_log) = std::env::var("CURSIVE_LOG") {
        match log::LevelFilter::from_str(&cursive_log) {
            Ok(filter_level) => {
                set_internal_filter_level(filter_level);
            }
            Err(e) => log::warn!("Could not parse CURSIVE_LOG: {}", e),
        }
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
        if metadata.target().starts_with("cursive_core::") {
            metadata.level() <= *INT_FILTER_LEVEL.read().unwrap()
        } else {
            metadata.level() <= *EXT_FILTER_LEVEL.read().unwrap()
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
    log::set_max_level((*INT_FILTER_LEVEL.read().unwrap()).max(*EXT_FILTER_LEVEL.read().unwrap()));
    // This will panic if `set_logger` was already called.
    log::set_logger(&CursiveLogger).unwrap();
}

/// Return a logger that stores records in cursive's log queue.
///
/// These logs can then be read by a [`DebugView`](crate::views::DebugView).
///
/// An easier alternative might be to use [`init()`].
pub fn get_logger() -> CursiveLogger {
    CursiveLogger
}

/// Adds `n` more entries to cursive's log queue.
///
/// Most of the time you don't need to use this directly.
pub fn reserve_logs(n: usize) {
    LOGS.lock().unwrap().reserve(n);
}
