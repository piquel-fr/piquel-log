//! Integration tests for explicit `log` bridging.

#![cfg(feature = "log")]

use piquel_log::Logger;

struct TestLogger;

impl log::Log for TestLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, _record: &log::Record<'_>) {}

    fn flush(&self) {}
}

#[test]
fn init_without_bridge_does_not_install_a_log_logger() {
    Logger::new()
        .init()
        .expect("init without bridge should succeed");

    log::set_boxed_logger(Box::new(TestLogger))
        .expect("log logger should still be installable when bridge is disabled");
}
