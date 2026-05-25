use std::fmt;

use time::OffsetDateTime;
use tracing::field::{Field, Visit};

use crate::sink::{FormattedEvent, FormatterConfig};

const TS_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// Render a tracing event into its final output form.
pub(crate) fn format_event(
    event: &tracing::Event<'_>,
    timestamp: OffsetDateTime,
    config: FormatterConfig,
) -> FormattedEvent {
    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);

    let level = *event.metadata().level();
    let target = event.metadata().target().to_owned();
    let timestamp = timestamp
        .format(TS_FORMAT)
        .unwrap_or_else(|_| timestamp.to_string());

    let level = format_level(level, config.ansi);
    let mut line = String::new();

    if config.timestamp {
        line.push_str(&timestamp);
        line.push(' ');
    }

    line.push_str(&level);

    if config.target {
        line.push(' ');
        line.push_str(&target);
        line.push(':');
    }

    if !visitor.message.is_empty() {
        line.push(' ');
        line.push_str(&visitor.message);
    }

    if !visitor.fields.is_empty() {
        line.push(' ');
        line.push_str(&visitor.fields.join(" "));
    }

    FormattedEvent { line }
}

fn format_level(level: tracing::Level, ansi: bool) -> String {
    let name = level.to_string();
    if !ansi {
        return format!("[{name}]");
    }

    let code = match level {
        tracing::Level::ERROR => 31,
        tracing::Level::WARN => 33,
        tracing::Level::INFO => 32,
        tracing::Level::DEBUG => 34,
        tracing::Level::TRACE => 36,
    };

    format!("[\x1b[{code}m{name}\x1b[0m]")
}

#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: Vec<String>,
}

impl EventVisitor {
    fn push_field(&mut self, field: &Field, rendered: String) {
        if field.name() == "message" {
            self.message = rendered;
        } else {
            self.fields.push(format!("{}={rendered}", field.name()));
        }
    }
}

impl Visit for EventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.push_field(field, value.to_owned());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push_field(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push_field(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push_field(field, value.to_string());
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.push_field(field, value.to_string());
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.push_field(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push_field(field, value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.push_field(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" && self.message.is_empty() {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(format!("{}={value:?}", field.name()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use time::OffsetDateTime;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::{Layer, layer::Context, prelude::*};

    use super::format_event;
    use crate::sink::FormatterConfig;

    #[derive(Clone)]
    struct CaptureLayer {
        config: FormatterConfig,
        lines: Arc<Mutex<Vec<String>>>,
    }

    impl CaptureLayer {
        fn new(config: FormatterConfig) -> Self {
            Self {
                config,
                lines: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn lines(&self) -> Vec<String> {
            self.lines.lock().expect("capture mutex poisoned").clone()
        }
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let rendered = format_event(event, OffsetDateTime::UNIX_EPOCH, self.config);
            self.lines
                .lock()
                .expect("capture mutex poisoned")
                .push(rendered.line);
        }
    }

    #[test]
    fn renders_plain_and_structured_fields() {
        let capture = CaptureLayer::new(FormatterConfig {
            ansi: false,
            target: true,
            timestamp: true,
        });
        let subscriber = tracing_subscriber::Registry::default().with(capture.clone());
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::info!(answer = 42, success = true, "hello world");

        let line = capture.lines().pop().expect("expected one rendered line");
        assert_eq!(
            line,
            "1970-01-01 00:00:00 [INFO] piquel_log::format::tests: hello world answer=42 success=true"
        );
    }

    #[test]
    fn renders_formatted_messages_without_ansi() {
        let capture = CaptureLayer::new(FormatterConfig {
            ansi: false,
            target: true,
            timestamp: true,
        });
        let subscriber = tracing_subscriber::Registry::default().with(capture.clone());
        let _guard = tracing::subscriber::set_default(subscriber);

        let subject = "world";
        tracing::warn!("hello {subject}");

        let line = capture.lines().pop().expect("expected one rendered line");
        assert!(line.contains("[WARN] piquel_log::format::tests: hello world"));
    }

    #[test]
    fn supports_disabling_target_and_timestamp() {
        let capture = CaptureLayer::new(FormatterConfig {
            ansi: false,
            target: false,
            timestamp: false,
        });
        let subscriber = tracing_subscriber::Registry::default().with(capture.clone());
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::debug!(user = "alice", "no decorations");

        let line = capture.lines().pop().expect("expected one rendered line");
        assert_eq!(line, "[DEBUG] no decorations user=alice");
    }

    #[test]
    fn supports_ansi_level_rendering() {
        let capture = CaptureLayer::new(FormatterConfig {
            ansi: true,
            target: false,
            timestamp: false,
        });
        let subscriber = tracing_subscriber::Registry::default().with(capture.clone());
        let _guard = tracing::subscriber::set_default(subscriber);

        tracing::error!("ansi");

        let line = capture.lines().pop().expect("expected one rendered line");
        assert_eq!(line, "[\u{1b}[31mERROR\u{1b}[0m] ansi");
    }
}
