//! Integration tests for global initialization behavior.

use piquel_log::{InitError, Logger};

#[test]
fn init_fails_when_called_twice() {
    Logger::new().init().expect("first init should succeed");

    let error = Logger::new().init().expect_err("second init should fail");

    assert!(matches!(error, InitError::AlreadyInitialized));
}
