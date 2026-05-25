use std::sync::Arc;

/// Rendering configuration shared by all sinks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FormatterConfig {
    pub(crate) ansi: bool,
    pub(crate) target: bool,
    pub(crate) timestamp: bool,
}

/// Final, sink-ready event representation.
#[derive(Debug, Clone)]
pub(crate) struct FormattedEvent {
    pub(crate) line: String,
}

pub(crate) type SharedSink = Arc<dyn Sink>;

pub(crate) trait Sink: Send + Sync {
    fn write(&self, event: &FormattedEvent);

    fn formatter_config(&self, base: FormatterConfig) -> FormatterConfig {
        base
    }
}
