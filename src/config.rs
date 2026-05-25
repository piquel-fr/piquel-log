use tracing_subscriber::{Registry, filter::LevelFilter, prelude::*, util::SubscriberInitExt};

use crate::{
    error::{BuildError, InitError},
    layer::BackendLayer,
    sink::FormatterConfig,
    sinks::console::ConsoleSink,
};

#[cfg(feature = "file")]
use crate::sinks::file::{FileSink, validate_file_config};

/// Builder for constructing and installing the crate's tracing backend.
#[derive(Debug, Clone)]
pub struct Logger {
    max_level: LevelFilter,
    ansi: bool,
    target: bool,
    timestamp: bool,
    #[cfg(feature = "file")]
    file: Option<FileConfig>,
    #[cfg(feature = "log")]
    log_bridge: bool,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            max_level: LevelFilter::INFO,
            ansi: true,
            target: true,
            timestamp: true,
            #[cfg(feature = "file")]
            file: None,
            #[cfg(feature = "log")]
            log_bridge: false,
        }
    }
}

impl Logger {
    /// Create a logger with sensible defaults.
    ///
    /// Defaults:
    /// - max level: `INFO`
    /// - ANSI colors: enabled
    /// - timestamps: enabled
    /// - targets: enabled
    /// - file output: disabled
    /// - `log` bridge: disabled
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the global maximum level applied during [`Self::init`].
    #[must_use]
    pub fn with_max_level(mut self, level: LevelFilter) -> Self {
        self.max_level = level;
        self
    }

    /// Enable or disable ANSI coloring for console output.
    #[must_use]
    pub fn with_ansi(mut self, enabled: bool) -> Self {
        self.ansi = enabled;
        self
    }

    /// Enable or disable including the event target in rendered output.
    #[must_use]
    pub fn with_target(mut self, enabled: bool) -> Self {
        self.target = enabled;
        self
    }

    /// Enable or disable timestamps in rendered output.
    #[must_use]
    pub fn with_timestamp(mut self, enabled: bool) -> Self {
        self.timestamp = enabled;
        self
    }

    /// Enable file output using the provided configuration.
    #[cfg(feature = "file")]
    #[cfg_attr(docsrs, doc(cfg(feature = "file")))]
    #[must_use]
    pub fn with_file(mut self, config: FileConfig) -> Self {
        self.file = Some(config);
        self
    }

    /// Enable or disable forwarding `log` records as `tracing` events during
    /// [`Self::init`].
    #[cfg(feature = "log")]
    #[cfg_attr(docsrs, doc(cfg(feature = "log")))]
    #[must_use]
    pub fn with_log_bridge(mut self, enabled: bool) -> Self {
        self.log_bridge = enabled;
        self
    }

    /// Build a composable [`BackendLayer`] without installing it globally.
    ///
    /// Use this when the application already assembles its own subscriber
    /// stack and only wants the backend output layer.
    ///
    /// Note that global max-level filtering is only installed by [`Self::init`].
    /// When using `build`, apply filtering in your own subscriber stack.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] when an optional sink cannot be constructed.
    pub fn build(self) -> Result<BackendLayer, BuildError> {
        let formatter = FormatterConfig {
            ansi: self.ansi,
            target: self.target,
            timestamp: self.timestamp,
        };

        let mut sinks = Vec::new();
        sinks.push(Box::new(ConsoleSink::default()) as _);

        #[cfg(feature = "file")]
        if let Some(file) = self.file {
            validate_file_config(&file)?;
            sinks.push(Box::new(FileSink::new(file)?) as _);
        }

        Ok(BackendLayer::new(formatter, sinks))
    }

    /// Build and install the backend as the global tracing subscriber.
    ///
    /// This method also installs the optional `log` bridge if enabled.
    ///
    /// # Errors
    ///
    /// Returns [`InitError::AlreadyInitialized`] if a global subscriber is
    /// already set, [`InitError::LogBridgeAlreadyInitialized`] if the `log`
    /// logger was already installed, or wraps a [`BuildError`] otherwise.
    pub fn init(self) -> Result<(), InitError> {
        let max_level = self.max_level;
        #[cfg(feature = "log")]
        let log_bridge = self.log_bridge;

        let layer = self.build().map_err(InitError::Build)?;

        #[cfg(feature = "log")]
        if log_bridge {
            tracing_log::LogTracer::init().map_err(|_| InitError::LogBridgeAlreadyInitialized)?;
        }

        Registry::default()
            .with(max_level)
            .with(layer)
            .try_init()
            .map_err(|_| InitError::AlreadyInitialized)
    }
}

/// File output configuration.
#[cfg(feature = "file")]
#[cfg_attr(docsrs, doc(cfg(feature = "file")))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConfig {
    pub(crate) directory: std::path::PathBuf,
    pub(crate) latest_file_name: String,
    pub(crate) session_file_prefix: Option<String>,
}

#[cfg(feature = "file")]
impl FileConfig {
    /// Create a file configuration rooted in `directory`.
    #[must_use]
    pub fn new(directory: impl Into<std::path::PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            latest_file_name: String::from("latest.log"),
            session_file_prefix: None,
        }
    }

    /// Change the path used for the rolling "latest" session file.
    #[must_use]
    pub fn with_latest_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.latest_file_name = file_name.into();
        self
    }

    /// Prefix the generated session file name.
    ///
    /// With prefix `app`, session files look like `app-2026-05-25_14-22-10.log`.
    #[must_use]
    pub fn with_session_file_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.session_file_prefix = Some(prefix.into());
        self
    }

    /// Return the configured output directory.
    #[must_use]
    pub fn directory(&self) -> &std::path::Path {
        &self.directory
    }

    /// Return the configured latest file name.
    #[must_use]
    pub fn latest_file_name(&self) -> &str {
        &self.latest_file_name
    }

    /// Return the configured session prefix, if any.
    #[must_use]
    pub fn session_file_prefix(&self) -> Option<&str> {
        self.session_file_prefix.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::filter::LevelFilter;

    use super::Logger;

    #[test]
    fn defaults_match_public_contract() {
        let logger = Logger::new();

        assert_eq!(logger.max_level, LevelFilter::INFO);
        assert!(logger.ansi);
        assert!(logger.target);
        assert!(logger.timestamp);

        #[cfg(feature = "log")]
        assert!(!logger.log_bridge);

        #[cfg(feature = "file")]
        assert!(logger.file.is_none());
    }

    #[test]
    fn max_level_is_stored() {
        let logger = Logger::new().with_max_level(LevelFilter::DEBUG);
        assert_eq!(logger.max_level, LevelFilter::DEBUG);
    }
}
