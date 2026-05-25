//! Integration tests for runtime backend updates.

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
    std::env::temp_dir().join(format!("piquel-log-runtime-test-{nanos}"))
}

#[test]
fn file_backend_can_be_added_after_layer_is_attached() {
    let directory = temp_logs_dir();
    let logger = Logger::new().with_ansi(false);
    let layer = logger.build().expect("layer should build");

    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!("console only");
    assert!(
        !directory.exists(),
        "file backend should not create files before it is added"
    );

    logger
        .add_file_backend(FileConfig::new(&directory).with_session_file_prefix("runtime"))
        .expect("file backend should be added");

    tracing::info!(answer = 42, "written after backend update");

    let latest = directory.join("latest.log");
    assert!(
        latest.exists(),
        "latest.log should exist after backend update"
    );

    let latest_contents = fs::read_to_string(&latest).expect("latest.log should be readable");
    assert!(latest_contents.contains("written after backend update answer=42"));
    assert!(
        !latest_contents.contains("console only"),
        "new backend should only receive events emitted after it was added"
    );

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
                    name.starts_with("runtime-")
                        && std::path::Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
                })
        })
        .collect::<Vec<_>>();

    assert_eq!(session_files.len(), 1, "one session file should be created");

    let _ = fs::remove_dir_all(directory);
}
