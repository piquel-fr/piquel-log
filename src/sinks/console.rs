use std::io::Write;

use crate::sink::{FormattedEvent, Sink};

/// Console output sink.
pub(crate) struct ConsoleSink;

impl Sink for ConsoleSink {
    fn write(&self, event: &FormattedEvent) {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{}", event.line);
    }
}
