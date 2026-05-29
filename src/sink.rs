use std::sync::Arc;

/// Rendering configuration shared by all sinks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FormatterConfig {
    pub(crate) ansi: bool,
    pub(crate) target: bool,
    pub(crate) timestamp: bool,
}

pub(crate) type SharedSink = Arc<dyn Sink>;

pub(crate) trait Sink: Send + Sync {
    fn write(&self, event: &str);

    fn formatter_config(&self, base: FormatterConfig) -> FormatterConfig {
        base
    }
}
