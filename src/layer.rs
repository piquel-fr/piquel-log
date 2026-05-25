use time::OffsetDateTime;
use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use crate::{
    format::format_event,
    sink::{FormatterConfig, Sink},
};

/// Public tracing layer produced by [`crate::Logger::build`].
pub struct BackendLayer {
    formatter: FormatterConfig,
    sinks: Vec<Box<dyn Sink>>,
}

impl BackendLayer {
    pub(crate) fn new(formatter: FormatterConfig, sinks: Vec<Box<dyn Sink>>) -> Self {
        Self { formatter, sinks }
    }
}

impl<S: Subscriber> Layer<S> for BackendLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let timestamp = OffsetDateTime::now_utc();
        for sink in &self.sinks {
            let rendered = format_event(event, timestamp, sink.formatter_config(self.formatter));
            sink.write(&rendered);
        }
    }
}
