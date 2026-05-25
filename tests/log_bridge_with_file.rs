//! Integration tests for `log` forwarding through `Logger::init`.

#![cfg(all(feature = "log", feature = "file"))]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use piquel_log::{FileConfig, Logger};

fn temp_logs_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("piquel-log-log-bridge-{nanos}"))
}

#[test]
fn init_with_bridge_forwards_log_records() {
    let directory = temp_logs_dir();
    let file = FileConfig::new(&directory).with_session_file_prefix("bridge");

    Logger::new()
        .with_ansi(false)
        .with_file(file)
        .with_log_bridge(true)
        .init()
        .expect("init with log bridge should succeed");

    log::warn!("bridged warning");
    log::info!("bridged info");

    let latest = directory.join("latest.log");
    let contents = fs::read_to_string(&latest).expect("latest.log should be readable");

    assert!(contents.contains("bridged warning"));
    assert!(contents.contains("bridged info"));

    let _ = fs::remove_dir_all(directory);
}
