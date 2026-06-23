use std::sync::Arc;

use time::OffsetDateTime;

use crate::{LogLevel, format::CapturedField};

/// Rendering configuration shared by all sinks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FormatterConfig {
    pub(crate) ansi: bool,
    pub(crate) target: bool,
    pub(crate) timestamp: bool,
}

/// Event data passed to sinks after formatting.
#[cfg_attr(not(feature = "store"), allow(dead_code))]
pub(crate) struct SinkEvent<'a> {
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) level: LogLevel,
    pub(crate) target: &'a str,
    pub(crate) message: &'a str,
    pub(crate) fields: &'a [CapturedField],
    pub(crate) rendered: &'a str,
}

impl<'a> SinkEvent<'a> {
    pub(crate) fn new(event: &'a crate::format::CapturedEvent, rendered: &'a str) -> Self {
        Self {
            timestamp: event.timestamp,
            level: event.level,
            target: &event.target,
            message: &event.message,
            fields: &event.fields,
            rendered,
        }
    }
}

pub(crate) type SharedSink = Arc<dyn Sink>;

pub(crate) trait Sink: Send + Sync {
    fn write(&self, event: SinkEvent<'_>);

    fn formatter_config(&self, base: FormatterConfig) -> FormatterConfig {
        base
    }
}
