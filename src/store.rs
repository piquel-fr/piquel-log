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

/// Builder-style filter for querying a [`LogStore`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFilter {
    max_level: Option<LogLevel>,
    target: Option<String>,
    target_prefix: Option<String>,
    text: Option<String>,
    limit: Option<usize>,
}

impl LogFilter {
    /// Create an empty filter that matches all entries.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Match entries at or above the provided severity threshold.
    #[must_use]
    pub fn with_max_level(mut self, level: LogLevel) -> Self {
        self.max_level = Some(level);
        self
    }

    /// Match entries with exactly this target.
    #[must_use]
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Match entries whose target starts with this prefix.
    #[must_use]
    pub fn with_target_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.target_prefix = Some(prefix.into());
        self
    }

    /// Match entries containing this case-sensitive text.
    ///
    /// The search checks messages, field names, field values, and rendered
    /// lines.
    #[must_use]
    pub fn containing_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Keep only the most recent `limit` matches.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn matches(&self, entry: &LogEntry) -> bool {
        if self
            .max_level
            .is_some_and(|max_level| entry.level > max_level)
        {
            return false;
        }

        if self
            .target
            .as_ref()
            .is_some_and(|target| entry.target != *target)
        {
            return false;
        }

        if self
            .target_prefix
            .as_ref()
            .is_some_and(|prefix| !entry.target.starts_with(prefix))
        {
            return false;
        }

        if self
            .text
            .as_ref()
            .is_some_and(|text| !entry_contains_text(entry, text))
        {
            return false;
        }

        true
    }
}

fn entry_contains_text(entry: &LogEntry, text: &str) -> bool {
    entry.message.contains(text)
        || entry.rendered.contains(text)
        || entry
            .fields
            .iter()
            .any(|field| field.name.contains(text) || field.value.contains(text))
}
