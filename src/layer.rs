use std::{collections::BTreeMap, sync::Arc};

use parking_lot::Mutex;
use time::OffsetDateTime;
use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use crate::{
    format::format_event,
    sink::{FormatterConfig, SharedSink},
};

pub(crate) type BackendId = usize;

/// Shared registry of sinks attached to a backend layer.
#[derive(Default)]
pub(crate) struct SinkRegistry {
    sinks: Mutex<BTreeMap<BackendId, SharedSink>>,
}

impl SinkRegistry {
    pub(crate) fn contains(&self, backend_id: BackendId) -> bool {
        self.sinks.lock().contains_key(&backend_id)
    }

    pub(crate) fn insert(&self, backend_id: BackendId, sink: SharedSink) {
        self.sinks.lock().insert(backend_id, sink);
    }

    pub(crate) fn remove(&self, backend_id: BackendId) {
        self.sinks.lock().remove(&backend_id);
    }

    fn snapshot(&self) -> Vec<SharedSink> {
        self.sinks.lock().values().cloned().collect()
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
