//! Small, composable backend initialization for [`tracing`].
//!
//! `piquel-log` is intended for applications that want a straightforward
//! way to install a `tracing` backend without exposing a large custom API.
//! The crate enables console output by default, allows the console sink to be
//! disabled, and can optionally support file output and `log` crate
//! interoperability behind Cargo features.
//!
//! # Quick start
//!
//! ```rust
//! use piquel_log::Logger;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! Logger::new().init()?;
//! tracing::info!("hello from tracing");
//! # Ok(())
//! # }
//! ```
//!
//! # Disable console output
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use piquel_log::Logger;
//!
//! Logger::new().with_console(false).init()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Existing subscriber stacks
//!
//! If your application already builds a `tracing_subscriber` registry, use
//! [`Logger::build`] and attach the returned [`BackendLayer`] yourself.
//!
//! ```rust
//! use piquel_log::Logger;
//! use tracing_subscriber::{filter::LevelFilter, prelude::*, Registry};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let backend = Logger::new().build()?;
//! let subscriber = Registry::default().with(LevelFilter::INFO).with(backend);
//! let _guard = tracing::subscriber::set_default(subscriber);
//! tracing::info!("hello from a custom stack");
//! # Ok(())
//! # }
//! ```
//!
//! # `log` interoperability
//!
//! When the `log` feature is enabled, [`Logger::with_log_bridge`] can install
//! `tracing_log::LogTracer` during [`Logger::init`] so that `log` records are
//! re-emitted as `tracing` events.
//!
//! ```rust
//! # #[cfg(feature = "log")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use piquel_log::Logger;
//!
//! Logger::new().with_log_bridge(true).init()?;
//! log::warn!("bridged from log");
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "log"))]
//! # fn main() {}
//! ```
//!
//! # Feature matrix
//!
//! - default: console backend only
//! - `Logger::with_console(false)`: disable the console sink
//! - `file`: configurable file output
//! - `Logger::add_file_backend(...)`: add a file backend at runtime
//! - `store`: queryable in-memory log storage
//! - `Logger::add_store_backend(...)`: add a store backend at runtime
//! - `log`: explicit `log` to `tracing` bridge during `init`
//! - `full`: enables `file`, `log`, and `store`
//!
//! # Queryable log store
//!
//! The `store` feature enables a thread-safe append-only in-memory backend
//! that captures structured log entries for later querying.
//!
//! ```rust
//! # #[cfg(feature = "store")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use piquel_log::{LogFilter, LogLevel, LogStore, Logger};
//!
//! let store = LogStore::new();
//!
//! Logger::new()
//!     .with_console(false)
//!     .with_store(store.clone())
//!     .init()?;
//!
//! tracing::warn!(target: "app::db", user = "alice", "slow query");
//!
//! let entries = store.query(
//!     &LogFilter::new()
//!         .with_max_level(LogLevel::Warn)
//!         .with_target_prefix("app::"),
//! );
//! assert_eq!(entries.len(), 1);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "store"))]
//! # fn main() {}
//! ```
//!
//! # Non-goals for v0.1
//!
//! - target allowlists or message filters
//! - file rotation or retention policies
//! - exposing individual internal sink/layer types
//!
//! # Runtime backend updates
//!
//! A [`Logger`] can keep the same backend layer attached while adding new
//! sinks later. For example, a file backend can be added after startup:
//!
//! ```rust
//! # #[cfg(feature = "file")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use piquel_log::{FileConfig, Logger};
//!
//! let logger = Logger::new();
//! logger.init()?;
//! logger.add_file_backend(FileConfig::new("logs"))?;
//! tracing::info!("also written to the file backend");
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "file"))]
//! # fn main() {}
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

mod config;
mod error;
mod format;
mod layer;
mod sink;
mod sinks;
#[cfg(feature = "store")]
mod store;

use std::fmt;

pub use crate::config::Logger;
pub use crate::error::{BuildError, InitError};
pub use crate::layer::BackendLayer;

#[cfg(feature = "file")]
#[cfg_attr(docsrs, doc(cfg(feature = "file")))]
pub use crate::config::FileConfig;

#[cfg(feature = "store")]
#[cfg_attr(docsrs, doc(cfg(feature = "store")))]
pub use crate::store::{LogEntry, LogField, LogFilter, LogStore};

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
/// This ordering is what powers level-threshold filtering: passing
/// `LogLevel::Warn` keeps `Error` and `Warn` (both ≤ `Warn` in severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Error conditions that usually require immediate attention.
    Error = 0,
    /// Warning conditions that may need investigation.
    Warn = 1,
    /// Normal operational information.
    Info = 2,
    /// Verbose diagnostic information for development.
    Debug = 3,
    /// The most detailed diagnostic information.
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
