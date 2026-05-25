use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};

use time::OffsetDateTime;

use crate::{
    config::FileConfig,
    error::BuildError,
    sink::{FormattedEvent, Sink},
};

const TS_FILE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");

pub(crate) fn validate_file_config(config: &FileConfig) -> Result<(), BuildError> {
    if config.latest_file_name.trim().is_empty() {
        return Err(BuildError::InvalidFileConfig(
            "latest file name cannot be empty",
        ));
    }

    if config
        .session_file_prefix
        .as_deref()
        .is_some_and(|prefix| prefix.trim().is_empty())
    {
        return Err(BuildError::InvalidFileConfig(
            "session file prefix cannot be empty",
        ));
    }

    Ok(())
}

/// File output sink writing to both a latest file and a session file.
pub(crate) struct FileSink {
    writers: Mutex<FileWriters>,
}

struct FileWriters {
    latest: File,
    session: File,
}

impl FileSink {
    pub(crate) fn new(config: FileConfig) -> Result<Self, BuildError> {
        std::fs::create_dir_all(&config.directory)?;

        let now = OffsetDateTime::now_utc();
        let latest_path = latest_path(&config);
        let session_path = session_path(&config, now);

        let latest = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(latest_path)?;

        let session = OpenOptions::new()
            .create(true)
            .append(true)
            .open(session_path)?;

        Ok(Self {
            writers: Mutex::new(FileWriters { latest, session }),
        })
    }
}

impl Sink for FileSink {
    fn write(&self, event: &FormattedEvent) {
        if let Ok(mut writers) = self.writers.lock() {
            let _ = writeln!(writers.latest, "{}", event.line);
            let _ = writeln!(writers.session, "{}", event.line);
        }
    }
}

pub(crate) fn latest_path(config: &FileConfig) -> PathBuf {
    config.directory.join(&config.latest_file_name)
}

pub(crate) fn session_path(config: &FileConfig, timestamp: OffsetDateTime) -> PathBuf {
    let timestamp = timestamp
        .format(TS_FILE_FORMAT)
        .unwrap_or_else(|_| String::from("unknown"));

    let file_name = match &config.session_file_prefix {
        Some(prefix) => format!("{prefix}-{timestamp}.log"),
        None => format!("{timestamp}.log"),
    };

    config.directory.join(file_name)
}

#[cfg(test)]
mod tests {
    use time::{Date, Month, PrimitiveDateTime, Time};

    use super::{latest_path, session_path, validate_file_config};
    use crate::{BuildError, FileConfig};

    fn sample_timestamp() -> time::OffsetDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::May, 25).expect("valid date"),
            Time::from_hms(9, 40, 15).expect("valid time"),
        )
        .assume_utc()
    }

    #[test]
    fn latest_file_uses_configured_name() {
        let config = FileConfig::new("logs").with_latest_file_name("current.log");
        assert_eq!(
            latest_path(&config),
            std::path::PathBuf::from("logs/current.log")
        );
    }

    #[test]
    fn session_file_uses_timestamp_and_prefix() {
        let config = FileConfig::new("logs").with_session_file_prefix("app");
        assert_eq!(
            session_path(&config, sample_timestamp()),
            std::path::PathBuf::from("logs/app-2026-05-25_09-40-15.log")
        );
    }

    #[test]
    fn rejects_empty_file_names() {
        let config = FileConfig::new("logs").with_latest_file_name("   ");
        match validate_file_config(&config) {
            Err(BuildError::InvalidFileConfig(message)) => {
                assert_eq!(message, "latest file name cannot be empty");
            }
            other => panic!("unexpected validation result: {other:?}"),
        }
    }
}
