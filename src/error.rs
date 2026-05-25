use thiserror::Error;

/// Errors returned while constructing backend components.
#[derive(Debug, Error)]
pub enum BuildError {
    /// File output configuration is invalid.
    #[cfg(feature = "file")]
    #[error("invalid file configuration: {0}")]
    InvalidFileConfig(&'static str),

    /// File sink setup failed.
    #[cfg(feature = "file")]
    #[error("file sink I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors returned while installing the backend globally.
#[derive(Debug, Error)]
pub enum InitError {
    /// A global tracing subscriber is already installed.
    #[error("a global tracing subscriber is already installed")]
    AlreadyInitialized,

    /// The `log` crate already has a global logger installed.
    #[cfg(feature = "log")]
    #[cfg_attr(docsrs, doc(cfg(feature = "log")))]
    #[error("the global log bridge is already installed")]
    LogBridgeAlreadyInitialized,

    /// Building the backend failed before installation.
    #[error(transparent)]
    Build(#[from] BuildError),
}
