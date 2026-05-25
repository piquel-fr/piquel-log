use std::sync::{Arc, Mutex};

use time::OffsetDateTime;
use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use crate::{
    format::format_event,
    sink::{FormatterConfig, SharedSink},
};

/// Shared registry of sinks attached to a backend layer.
#[derive(Default)]
pub(crate) struct SinkRegistry {
    sinks: Mutex<Vec<SharedSink>>,
}

impl SinkRegistry {
    pub(crate) fn push(&self, sink: SharedSink) {
        match self.sinks.lock() {
            Ok(mut sinks) => sinks.push(sink),
            Err(poisoned) => poisoned.into_inner().push(sink),
        }
    }

    fn snapshot(&self) -> Vec<SharedSink> {
        match self.sinks.lock() {
            Ok(sinks) => sinks.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

/// Public tracing layer produced by [`crate::Logger::build`].
pub struct BackendLayer {
    formatter: FormatterConfig,
    sinks: Arc<SinkRegistry>,
}

impl BackendLayer {
    pub(crate) fn new(formatter: FormatterConfig, sinks: Arc<SinkRegistry>) -> Self {
        Self { formatter, sinks }
    }
}

impl<S: Subscriber> Layer<S> for BackendLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let timestamp = OffsetDateTime::now_utc();

        for sink in self.sinks.snapshot() {
            let rendered = format_event(event, timestamp, sink.formatter_config(self.formatter));
            sink.write(&rendered);
        }
    }
}
