//! Integration tests for the queryable in-memory log store.

#![cfg(feature = "store")]

use piquel_log::{LogFilter, LogLevel, LogStore, Logger};
use tracing_subscriber::prelude::*;

fn attach_logger(logger: &Logger) -> tracing::dispatcher::DefaultGuard {
    let layer = logger.build().expect("layer should build");
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::filter::LevelFilter::TRACE)
        .with(layer);
    tracing::subscriber::set_default(subscriber)
}

#[test]
fn store_backend_captures_structured_entries() {
    let store = LogStore::new();
    let logger = Logger::new()
        .with_console(false)
        .with_ansi(true)
        .with_store(store.clone());
    let _guard = attach_logger(&logger);

    tracing::info!(
        target: "app::store",
        user = "alice",
        answer = 42,
        "hello store"
    );

    let entries = store.entries();
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry.level(), LogLevel::Info);
    assert_eq!(entry.target(), "app::store");
    assert_eq!(entry.message(), "hello store");
    assert!(
        entry
            .fields()
            .iter()
            .any(|field| field.name() == "user" && field.value() == "alice")
    );
    assert!(
        entry
            .fields()
            .iter()
            .any(|field| field.name() == "answer" && field.value() == "42")
    );
    assert!(entry.rendered().contains("hello store"));
    assert!(entry.rendered().contains("answer=42"));
    assert!(!entry.rendered().contains("\x1b["));
}

#[test]
fn store_query_filters_by_level_threshold() {
    let store = LogStore::new();
    let logger = Logger::new().with_console(false).with_store(store.clone());
    let _guard = attach_logger(&logger);

    tracing::error!("error");
    tracing::warn!("warn");
    tracing::info!("info");
    tracing::debug!("debug");

    let entries = store.query(&LogFilter::new().with_max_level(LogLevel::Warn));
    let levels = entries
        .iter()
        .map(piquel_log::LogEntry::level)
        .collect::<Vec<_>>();

    assert_eq!(levels, vec![LogLevel::Error, LogLevel::Warn]);
}

#[test]
fn store_query_filters_by_target_and_prefix() {
    let store = LogStore::new();
    let logger = Logger::new().with_console(false).with_store(store.clone());
    let _guard = attach_logger(&logger);

    tracing::info!(target: "app::db", "database");
    tracing::info!(target: "app::http", "http");
    tracing::info!(target: "other", "other");

    let exact = store.query(&LogFilter::new().with_target("app::db"));
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].target(), "app::db");

    let prefixed = store.query(&LogFilter::new().with_target_prefix("app::"));
    let targets = prefixed
        .iter()
        .map(piquel_log::LogEntry::target)
        .collect::<Vec<_>>();
    assert_eq!(targets, vec!["app::db", "app::http"]);
}

#[test]
fn store_query_filters_by_text() {
    let store = LogStore::new();
    let logger = Logger::new().with_console(false).with_store(store.clone());
    let _guard = attach_logger(&logger);

    tracing::info!(
        target: "app::text",
        account_id = "alice",
        status = "ok",
        "needle message"
    );

    for text in ["needle", "account_id", "alice", "status=ok"] {
        let entries = store.query(&LogFilter::new().containing_text(text));
        assert_eq!(entries.len(), 1, "expected one match for {text}");
    }
}

#[test]
fn store_query_limit_returns_recent_matches_in_chronological_order() {
    let store = LogStore::new();
    let logger = Logger::new().with_console(false).with_store(store.clone());
    let _guard = attach_logger(&logger);

    tracing::info!("match one");
    tracing::info!("match two");
    tracing::info!("match three");

    let entries = store.query(&LogFilter::new().containing_text("match").with_limit(2));
    let messages = entries
        .iter()
        .map(piquel_log::LogEntry::message)
        .collect::<Vec<_>>();

    assert_eq!(messages, vec!["match two", "match three"]);
}

#[test]
fn store_backend_can_be_added_after_layer_is_attached() {
    let store = LogStore::new();
    let logger = Logger::new().with_console(false);
    let _guard = attach_logger(&logger);

    tracing::info!("before store");
    logger.add_store_backend(store.clone());
    tracing::info!("after store");

    let entries = store.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message(), "after store");
}

#[test]
fn multiple_store_backends_receive_independent_appends() {
    let first = LogStore::new();
    let second = LogStore::new();
    let logger = Logger::new()
        .with_console(false)
        .with_store(first.clone())
        .with_store(second.clone());
    let _guard = attach_logger(&logger);

    tracing::info!(target: "app::multi", user = "alice", "fan out");

    let first_entries = first.entries();
    let second_entries = second.entries();

    assert_eq!(first_entries.len(), 1);
    assert_eq!(second_entries.len(), 1);
    assert_eq!(first_entries, second_entries);
}
