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

impl<S> Layer<S> for BackendLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let rendered = format_event(event, OffsetDateTime::now_utc(), self.formatter);
        for sink in &self.sinks {
            sink.write(&rendered);
        }
    }
}
