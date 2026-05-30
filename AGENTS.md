# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 library crate for initializing and evolving `tracing`
backends. Public exports are defined in `src/lib.rs`. Core configuration and
the public `Logger` handle live in `src/config.rs`; formatting lives in
`src/format.rs`; subscriber integration plus the shared sink registry live in
`src/layer.rs`; sink traits and shared sink aliases live in `src/sink.rs`;
and error types live in `src/error.rs`. Concrete sinks are under
`src/sinks/`, currently `console.rs` and feature-gated `file.rs`. Integration
tests live in `tests/` and are organized by behavior, for example
`tests/formatting.rs`, `tests/file_output.rs`, and `tests/runtime_backends.rs`.

## Build, Test, and Development Commands

- `cargo check` validates the crate quickly without producing release
  artifacts.
- `cargo test` runs the default feature set and integration tests.
- `cargo test --all-features` runs tests with `file`, `log`, and `full`
  behavior enabled.
- `cargo clippy --all-features --all-targets` checks lint quality across
  library and tests.
- `cargo fmt` formats the codebase with rustfmt.
- `cargo doc --all-features --no-deps` builds local API documentation.

## Coding Style & Naming Conventions

Use standard rustfmt formatting and four-space indentation. Prefer small,
focused modules that mirror the existing layout. Types and traits use
`UpperCamelCase`; functions, methods, modules, and feature names use
`snake_case`. Keep public API names direct, such as `Logger`, `FileConfig`,
`BuildError`, and `InitError`. The manifest forbids unsafe code, warns on
missing docs, and enables Clippy `pedantic` plus `unwrap_used`, so avoid
`unwrap()` in library code and document public items.

## Testing Guidelines

Add integration tests under `tests/` for externally visible behavior. Name
test files after the feature or workflow being exercised, and name individual
tests with clear `snake_case` behavior descriptions. When changing feature
gates, run both `cargo test` and `cargo test --all-features`. File-output
changes should verify generated log paths and contents without depending on
shared state between tests.

## Commit & Pull Request Guidelines

Recent commits use short, imperative, lowercase messages such as
`add integration tests` and `fix outputting color to text`; follow that style
for single-purpose commits. Pull requests should include a concise summary,
the relevant tests or checks run, and any feature-flag impact. Link related
issues when available. Include terminal output snippets only when they clarify
failures or behavior changes; screenshots are generally unnecessary for this
library crate.

## Security & Configuration Tips

Keep optional behavior behind Cargo features. Do not introduce global logging
side effects outside documented initialization paths. Avoid adding runtime
dependencies unless they are needed by the core backend or an explicit feature.
