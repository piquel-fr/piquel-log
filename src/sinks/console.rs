use std::io::Write;

use crate::sink::Sink;

/// Console output sink.
pub(crate) struct ConsoleSink;

impl Sink for ConsoleSink {
    fn write(&self, event: &str) {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{event}");
    }
}
