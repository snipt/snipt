# Snipt Daemon

The background service component for the Snipt application.

## Overview

Snipt Daemon provides background services for Snipt, enabling clipboard monitoring, hotkey detection, and automatic snippet triggering even when the main application isn't actively being used.

## Features

- Clipboard monitoring and management
- Global hotkey detection and handling
- Automated snippet expansion
- Low resource usage
- Cross-platform compatibility

## Usage

The daemon is typically started through the Snipt CLI:

```bash
snipt daemon start
```

Or programmatically:

```rust
use snipt_daemon::Daemon;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let daemon = Daemon::new()?;
    daemon.start()?;
    Ok(())
}
```

## Configuration

The daemon behavior can be configured through:

- Configuration files
- Environment variables
- Command line arguments

## License

MIT 