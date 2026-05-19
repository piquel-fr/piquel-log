//! A simple & extensible `tracing` backend.

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::fmt;

/// Severity level of a log entry.
///
/// Variants are ordered by severity so that comparisons work intuitively:
/// `Error` is the **most** severe and `Trace` is the **least**.
///
/// ```rust
/// use piquel_log::LogLevel;
///
/// assert!(LogLevel::Error < LogLevel::Warn);
/// assert!(LogLevel::Warn  < LogLevel::Info);
/// assert!(LogLevel::Info  < LogLevel::Debug);
/// assert!(LogLevel::Debug < LogLevel::Trace);
/// ```
///
/// This ordering is what powers [`Filter::min_level`]: passing
/// `LogLevel::Warn` keeps `Error` and `Warn` (both ≤ `Warn` in severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl From<LogLevel> for tracing::metadata::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

impl From<&tracing::Level> for LogLevel {
    fn from(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::ERROR => Self::Error,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::INFO => Self::Info,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::TRACE => Self::Trace,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Error => "ERROR",
                Self::Warn => "WARN",
                Self::Info => "INFO",
                Self::Debug => "DEBUG",
                Self::Trace => "TRACE",
            }
        )
    }
}
