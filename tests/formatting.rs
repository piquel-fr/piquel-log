//! Integration tests for attaching the backend layer to an existing subscriber.

use piquel_log::Logger;
use tracing_subscriber::prelude::*;

#[test]
fn build_returns_attachable_layer() {
    let layer = Logger::new().build().expect("layer should build");
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!(answer = 42, "layer attached");
}
