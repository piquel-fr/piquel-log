use std::io::Write;

use crate::sink::{Sink, SinkEvent};

/// Console output sink.
pub(crate) struct ConsoleSink;

impl Sink for ConsoleSink {
    fn write(&self, event: SinkEvent<'_>) {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{}", event.rendered);
    }
}
