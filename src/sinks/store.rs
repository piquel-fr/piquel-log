use crate::{
    sink::{FormatterConfig, Sink, SinkEvent},
    store::{LogEntry, LogStore},
};

/// In-memory store sink.
pub(crate) struct StoreSink {
    store: LogStore,
}

impl StoreSink {
    pub(crate) fn new(store: LogStore) -> Self {
        Self { store }
    }
}

impl Sink for StoreSink {
    fn formatter_config(&self, mut base: FormatterConfig) -> FormatterConfig {
        base.ansi = false;
        base
    }

    fn write(&self, event: SinkEvent<'_>) {
        self.store.push(LogEntry::from_sink_event(&event));
    }
}
