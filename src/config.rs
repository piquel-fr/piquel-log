use parking_lot::Mutex;
use std::sync::Arc;

use tracing_subscriber::{Registry, filter::LevelFilter, prelude::*, util::SubscriberInitExt};

use crate::{
    LogLevel,
    error::{BuildError, InitError},
    layer::{BackendId, BackendLayer, SinkRegistry},
    sink::FormatterConfig,
    sinks::console::ConsoleSink,
};

#[cfg(feature = "file")]
use crate::sinks::file::{FileSink, validate_file_config};

/// If you want to add a backend to the library, add it to this enum.
/// The compiler will tell you what you need to updated.
#[derive(Debug, Clone)]
enum BackendKind {
    Console,
    #[cfg(feature = "file")]
    File(FileConfig),
}

#[derive(Debug, Clone)]
struct BackendSpec {
    id: BackendId,
    kind: BackendKind,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct LoggerState {
    max_level: LogLevel,
    ansi: bool,
    target: bool,
    timestamp: bool,
    backend_specs: Vec<BackendSpec>,
    next_backend_id: BackendId,
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
            backend_specs: vec![BackendSpec {
                id: 0,
                kind: BackendKind::Console,
            }],
            next_backend_id: 1,
            #[cfg(feature = "log")]
            log_bridge: false,
        }
    }
}

/// Builder and runtime handle for the crate's tracing backend.
#[derive(Clone, Default)]
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
            .field("next_backend_id", &state.next_backend_id)
            .finish_non_exhaustive()
    }
}

impl Logger {
    /// Create a logger with sensible defaults.
    ///
    /// Defaults:
    /// - max level: `INFO`
    /// - console output: enabled
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

    /// Enable or disable the console sink.
    ///
    /// Disabling the console sink removes it from the live backend stack.
    #[must_use]
    pub fn with_console(self, enabled: bool) -> Self {
        self.set_console_enabled(enabled);
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
        self.stage_backend(BackendKind::File(config));
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
        let sink = Arc::new(Self::build_file_sink(&config)?);
        let backend_id = self.stage_backend(BackendKind::File(config));
        self.sinks.insert(backend_id, sink);
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
        let specs = self.lock_state().backend_specs.clone();

        for spec in specs {
            if self.sinks.contains(spec.id) {
                continue;
            }

            let sink = Self::build_sink(&spec.kind)?;
            self.sinks.insert(spec.id, sink);
        }

        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn build_sink(kind: &BackendKind) -> Result<crate::sink::SharedSink, BuildError> {
        Ok(match kind {
            BackendKind::Console => Arc::new(ConsoleSink),
            #[cfg(feature = "file")]
            BackendKind::File(config) => Arc::new(Self::build_file_sink(config)?),
        })
    }

    #[cfg(feature = "file")]
    fn build_file_sink(config: &FileConfig) -> Result<FileSink, BuildError> {
        validate_file_config(config)?;
        FileSink::new(config)
    }

    fn set_console_enabled(&self, enabled: bool) {
        let backend_id = self.update_state(|state| {
            let console_index = state
                .backend_specs
                .iter()
                .position(|spec| matches!(spec.kind, BackendKind::Console));

            match (enabled, console_index) {
                (true, Some(index)) => Some(state.backend_specs[index].id),
                (true, None) => {
                    let id = state.next_backend_id;
                    state.next_backend_id += 1;
                    state.backend_specs.insert(
                        0,
                        BackendSpec {
                            id,
                            kind: BackendKind::Console,
                        },
                    );
                    Some(id)
                }
                (false, Some(index)) => Some(state.backend_specs.remove(index).id),
                (false, None) => None,
            }
        });

        match (enabled, backend_id) {
            (true, Some(id)) if !self.sinks.contains(id) => {
                self.sinks.insert(id, Arc::new(ConsoleSink));
            }
            (false, Some(id)) => self.sinks.remove(id),
            _ => {}
        }
    }

    // This is used by other cargo features
    #[allow(unused)]
    fn stage_backend(&self, kind: BackendKind) -> BackendId {
        self.update_state(|state| {
            let id = state.next_backend_id;
            state.next_backend_id += 1;
            state.backend_specs.push(BackendSpec { id, kind });
            id
        })
    }

    fn update_state<T>(&self, update: impl FnOnce(&mut LoggerState) -> T) -> T {
        let mut state = self.state.lock();
        update(&mut state)
    }

    fn lock_state(&self) -> parking_lot::MutexGuard<'_, LoggerState> {
        self.state.lock()
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
    use super::{BackendKind, Logger};
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
        assert!(matches!(state.backend_specs[0].kind, BackendKind::Console));

        #[cfg(feature = "log")]
        assert!(!state.log_bridge);
    }

    #[test]
    fn max_level_is_stored() {
        let logger = Logger::new().with_max_level(LogLevel::Debug);
        assert_eq!(logger.lock_state().max_level, LogLevel::Debug);
    }

    #[test]
    fn console_sink_can_be_disabled() {
        let logger = Logger::new().with_console(false);
        assert!(logger.lock_state().backend_specs.is_empty());
    }

    #[test]
    fn console_sink_can_be_reenabled() {
        let logger = Logger::new().with_console(false).with_console(true);
        let state = logger.lock_state();

        assert_eq!(state.backend_specs.len(), 1);
        assert!(matches!(state.backend_specs[0].kind, BackendKind::Console));
    }
}
