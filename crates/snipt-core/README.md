# Snipt Core

The core functionality library for the Snipt application.

## Overview

This crate contains the core domain logic, data structures, and algorithms used by all other Snipt components. It's designed to be a dependency of the CLI, server, UI, and daemon components, providing common functionality without containing any application-specific code.

## Features

- Core data structures for snippet management
- Shared utilities and helper functions
- Common traits and interfaces
- Error handling definitions

## Usage

This library is not meant to be used directly, but rather as a dependency of other Snipt components.

```rust
use snipt_core::{Snippet, SnippetManager};

fn example() {
    let manager = SnippetManager::new();
    // Use the core functionality
}
```

## License

MIT 