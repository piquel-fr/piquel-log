use time::OffsetDateTime;

use crate::{LogLevel, sink::SinkEvent};

/// A captured structured log event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    timestamp: OffsetDateTime,
    level: LogLevel,
    target: String,
    message: String,
    fields: Vec<LogField>,
    rendered: String,
}

impl LogEntry {
    pub(crate) fn from_sink_event(event: &SinkEvent<'_>) -> Self {
        Self {
            timestamp: event.timestamp,
            level: event.level,
            target: event.target.to_owned(),
            message: event.message.to_owned(),
            fields: event.fields.iter().map(LogField::from).collect(),
            rendered: event.rendered.to_owned(),
        }
    }

    /// Return the timestamp captured for this entry.
    #[must_use]
    pub fn timestamp(&self) -> OffsetDateTime {
        self.timestamp
    }

    /// Return the entry severity.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Return the entry target.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Return the entry message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the structured fields captured for this entry.
    #[must_use]
    pub fn fields(&self) -> &[LogField] {
        &self.fields
    }

    /// Return the sink-rendered line for this entry.
    #[must_use]
    pub fn rendered(&self) -> &str {
        &self.rendered
    }
}

/// A captured structured log field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogField {
    name: String,
    value: String,
}

impl LogField {
    /// Return the field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the string-rendered field value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl From<&crate::format::CapturedField> for LogField {
    fn from(field: &crate::format::CapturedField) -> Self {
        Self {
            name: field.name.clone(),
            value: field.value.clone(),
        }
    }
}
