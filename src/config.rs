use std::sync::{Arc, Mutex};

use tracing_subscriber::{Registry, filter::LevelFilter, prelude::*, util::SubscriberInitExt};

use crate::{
    LogLevel,
    error::{BuildError, InitError},
    layer::{BackendLayer, SinkRegistry},
    sink::FormatterConfig,
    sinks::console::ConsoleSink,
};

#[cfg(feature = "file")]
use crate::sinks::file::{FileSink, validate_file_config};

/// If you want to add a backend to the library, add it to this enum.
/// The compiler will tell you what you need to updated.
#[derive(Debug, Clone)]
enum BackendSpec {
    Console,
    #[cfg(feature = "file")]
    File(FileConfig),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct LoggerState {
    max_level: LogLevel,
    ansi: bool,
    target: bool,
    timestamp: bool,
    backend_specs: Vec<BackendSpec>,
    realized_backends: usize,
    #[cfg(feature = "log")]
    log_bridge: bool,
}

impl Default for LoggerState {
    fn default() -> Self {
        Self {
            max_level: LogLevel::Info,
            ansi: true,
            target: true,
            timestamp: true,
            backend_specs: vec![BackendSpec::Console],
            realized_backends: 0,
            #[cfg(feature = "log")]
            log_bridge: false,
        }
    }
}

/// Builder and runtime handle for the crate's tracing backend.
#[derive(Clone)]
pub struct Logger {
    state: Arc<Mutex<LoggerState>>,
    sinks: Arc<SinkRegistry>,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.lock_state();
        f.debug_struct("Logger")
            .field("max_level", &state.max_level)
            .field("ansi", &state.ansi)
            .field("target", &state.target)
            .field("timestamp", &state.timestamp)
            .field("backend_specs", &state.backend_specs)
            .field("realized_backends", &state.realized_backends)
            .finish_non_exhaustive()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(LoggerState::default())),
            sinks: Arc::new(SinkRegistry::default()),
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
    pub fn with_max_level(self, level: LogLevel) -> Self {
        self.update_state(|state| state.max_level = level);
        self
    }

    /// Enable or disable ANSI coloring for console output.
    #[must_use]
    pub fn with_ansi(self, enabled: bool) -> Self {
        self.update_state(|state| state.ansi = enabled);
        self
    }

    /// Enable or disable including the event target in rendered output.
    #[must_use]
    pub fn with_target(self, enabled: bool) -> Self {
        self.update_state(|state| state.target = enabled);
        self
    }

    /// Enable or disable timestamps in rendered output.
    #[must_use]
    pub fn with_timestamp(self, enabled: bool) -> Self {
        self.update_state(|state| state.timestamp = enabled);
        self
    }

    /// Stage a file backend so the next [`Self::build`] or [`Self::init`]
    /// includes it in the active backend stack.
    #[cfg(feature = "file")]
    #[cfg_attr(docsrs, doc(cfg(feature = "file")))]
    #[must_use]
    pub fn with_file(self, config: FileConfig) -> Self {
        self.update_state(|state| state.backend_specs.push(BackendSpec::File(config)));
        self
    }

    /// Add a file backend to the active backend stack.
    ///
    /// The backend starts receiving events immediately after this method
    /// succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] when the file backend configuration is invalid
    /// or the sink cannot be constructed.
    #[cfg(feature = "file")]
    #[cfg_attr(docsrs, doc(cfg(feature = "file")))]
    pub fn add_file_backend(&self, config: FileConfig) -> Result<(), BuildError> {
        let sink = Self::build_file_sink(&config)?;

        self.update_state(|state| {
            state.backend_specs.push(BackendSpec::File(config));
            state.realized_backends += 1;
        });
        self.sinks.push(Arc::new(sink));

        Ok(())
    }

    /// Enable or disable forwarding `log` records as `tracing` events during
    /// [`Self::init`].
    #[cfg(feature = "log")]
    #[cfg_attr(docsrs, doc(cfg(feature = "log")))]
    #[must_use]
    pub fn with_log_bridge(self, enabled: bool) -> Self {
        self.update_state(|state| state.log_bridge = enabled);
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
    pub fn build(&self) -> Result<BackendLayer, BuildError> {
        self.ensure_realized_backends()?;

        Ok(BackendLayer::new(
            self.formatter_config(),
            Arc::clone(&self.sinks),
        ))
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
    pub fn init(&self) -> Result<(), InitError> {
        let max_level = self.lock_state().max_level;
        #[cfg(feature = "log")]
        let log_bridge = self.lock_state().log_bridge;

        let layer = self.build()?;

        #[cfg(feature = "log")]
        if log_bridge {
            tracing_log::LogTracer::init().map_err(|_| InitError::LogBridgeAlreadyInitialized)?;
        }

        Registry::default()
            .with(LevelFilter::from(max_level))
            .with(layer)
            .try_init()
            .map_err(|_| InitError::AlreadyInitialized)
    }

    fn formatter_config(&self) -> FormatterConfig {
        let state = self.lock_state();
        FormatterConfig {
            ansi: state.ansi,
            target: state.target,
            timestamp: state.timestamp,
        }
    }

    fn ensure_realized_backends(&self) -> Result<(), BuildError> {
        let missing_specs = {
            let state = self.lock_state();
            state.backend_specs[state.realized_backends..].to_vec()
        };

        for spec in missing_specs {
            let sink = Self::build_sink(&spec)?;
            self.sinks.push(sink);
            self.update_state(|state| state.realized_backends += 1);
        }

        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn build_sink(spec: &BackendSpec) -> Result<crate::sink::SharedSink, BuildError> {
        Ok(match spec {
            BackendSpec::Console => Arc::new(ConsoleSink),
            #[cfg(feature = "file")]
            BackendSpec::File(config) => Arc::new(Self::build_file_sink(config)?),
        })
    }

    #[cfg(feature = "file")]
    fn build_file_sink(config: &FileConfig) -> Result<FileSink, BuildError> {
        validate_file_config(config)?;
        FileSink::new(config)
    }

    fn update_state(&self, update: impl FnOnce(&mut LoggerState)) {
        match self.state.lock() {
            Ok(mut state) => update(&mut state),
            Err(poisoned) => update(&mut poisoned.into_inner()),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, LoggerState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
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
    use super::{BackendSpec, Logger};
    use crate::LogLevel;

    #[test]
    fn defaults_match_public_contract() {
        let logger = Logger::new();
        let state = logger.lock_state();

        assert_eq!(state.max_level, LogLevel::Info);
        assert!(state.ansi);
        assert!(state.target);
        assert!(state.timestamp);
        assert_eq!(state.backend_specs.len(), 1);
        assert!(matches!(state.backend_specs[0], BackendSpec::Console));

        #[cfg(feature = "log")]
        assert!(!state.log_bridge);
    }

    #[test]
    fn max_level_is_stored() {
        let logger = Logger::new().with_max_level(LogLevel::Debug);
        assert_eq!(logger.lock_state().max_level, LogLevel::Debug);
    }
}
