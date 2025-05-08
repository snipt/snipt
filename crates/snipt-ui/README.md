# Snipt UI

The terminal user interface components for the Snipt application.

## Overview

This crate provides the terminal UI functionality for Snipt, built with Crossterm and Ratatui. It includes components for displaying and interacting with snippets, search functionality, and status information in a terminal environment.

## Features

- Terminal-based user interface
- Interactive snippet management
- Search and filtering capabilities
- Keyboard shortcuts and command palette
- Customizable themes and layouts

## Usage

The UI components are typically used by the CLI application but can also be used programmatically:

```rust
use snipt_ui::{App, UIConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = UIConfig::default();
    let mut app = App::new(config);
    app.run()?;
    Ok(())
}
```

## Components

- `App`: The main application UI
- `Snippets`: Snippet list view
- `Editor`: Snippet editing interface
- `SearchBar`: Search functionality
- `StatusBar`: Status information

## License

MIT 