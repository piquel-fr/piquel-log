# piquel-log

Small, composable backend initialization for `tracing`.

`piquel-log` is a backend helper for applications that want:

- a simple `Logger::new().init()?` path
- a logger handle that can evolve the backend stack at runtime
- console output by default, with an option to disable it
- optional file output behind a feature flag
- optional `log` crate interoperability behind a feature flag
- a single layer that can be attached to an existing `tracing_subscriber` stack

## Features

- default: console output
- `Logger::with_console(false)`: disable the console sink
- `file`: file output with `latest.log` plus one session file per initialization
- `Logger::add_file_backend(...)`: add a file sink after initialization
- `log`: explicit `log` to `tracing` bridge during `init`
- `full`: enables `file` and `log`

## Quick start

```rust
use piquel_log::Logger;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
Logger::new().init()?;
tracing::info!("hello from tracing");
# Ok(())
# }
```

## Disable console output

```rust
# #[cfg(feature = "file")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use piquel_log::{FileConfig, Logger};

let file = FileConfig::new("logs").with_session_file_prefix("app");
Logger::new()
    .with_console(false)
    .with_file(file)
    .init()?;
# Ok(())
# }
# #[cfg(not(feature = "file"))]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use piquel_log::Logger;

Logger::new().with_console(false).init()?;
# Ok(())
# }
```

## Custom subscriber stacks

```rust
use piquel_log::Logger;
use tracing_subscriber::{filter::LevelFilter, prelude::*, Registry};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let backend = Logger::new().build()?;
let subscriber = Registry::default().with(LevelFilter::INFO).with(backend);
let _guard = tracing::subscriber::set_default(subscriber);
tracing::info!("hello from a custom stack");
# Ok(())
# }
```

## File output

```rust
# #[cfg(feature = "file")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use piquel_log::{FileConfig, Logger};

let file = FileConfig::new("logs").with_session_file_prefix("app");
Logger::new().with_file(file).init()?;
# Ok(())
# }
# #[cfg(not(feature = "file"))]
# fn main() {}
```

## Runtime backend updates

```rust
# #[cfg(feature = "file")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use piquel_log::{FileConfig, Logger};

let logger = Logger::new();
logger.init()?;

logger.add_file_backend(
    FileConfig::new("logs").with_session_file_prefix("runtime"),
)?;

tracing::info!("also written to the runtime file backend");
# Ok(())
# }
# #[cfg(not(feature = "file"))]
# fn main() {}
```

## `log` crate interoperability

```rust
# #[cfg(feature = "log")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use piquel_log::Logger;

Logger::new().with_log_bridge(true).init()?;
log::warn!("bridged from log");
# Ok(())
# }
# #[cfg(not(feature = "log"))]
# fn main() {}
```
