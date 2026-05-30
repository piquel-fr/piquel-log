//! Integration tests for file output.

#![cfg(feature = "file")]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use piquel_log::{FileConfig, Logger};
use tracing_subscriber::prelude::*;

fn temp_logs_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("piquel-log-test-{nanos}"))
}

#[test]
fn file_sink_writes_latest_and_session_files() {
    let directory = temp_logs_dir();
    let file = FileConfig::new(&directory).with_session_file_prefix("app");
    let layer = Logger::new()
        .with_console(false)
        .with_ansi(false)
        .with_file(file)
        .build()
        .expect("file-enabled layer should build");

    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!(code = 7, "written to files");

    let latest = directory.join("latest.log");
    assert!(latest.exists(), "latest.log should exist");

    let entries = fs::read_dir(&directory)
        .expect("directory should be readable")
        .map(|entry| entry.expect("directory entry should be valid").path())
        .collect::<Vec<_>>();

    let session_files = entries
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("app-")
                        && std::path::Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
                })
        })
        .collect::<Vec<_>>();

    assert_eq!(session_files.len(), 1, "one session file should be created");

    let latest_contents = fs::read_to_string(&latest).expect("latest.log should be readable");
    assert!(latest_contents.contains("written to files code=7"));
    assert!(
        !latest_contents.contains("\u{1b}["),
        "latest.log should not contain ANSI escape sequences"
    );

    let session_contents =
        fs::read_to_string(session_files[0]).expect("session file should be readable");
    assert!(session_contents.contains("written to files code=7"));
    assert!(
        !session_contents.contains("\u{1b}["),
        "session log should not contain ANSI escape sequences"
    );

    let _ = fs::remove_dir_all(directory);
}
