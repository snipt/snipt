# Scribe Architecture

This document outlines the architecture and design decisions behind the Scribe text expansion tool.

## Overall Structure

Scribe follows a modular architecture with clear separation of concerns:

```
scribe/
├── src/
│   ├── config.rs        # Configuration handling
│   ├── daemon.rs        # Background daemon functionality
│   ├── error.rs         # Error types and handling
│   ├── expansion.rs     # Text expansion logic
│   ├── interactive.rs   # Interactive terminal UI for adding snippets
│   ├── keyboard.rs      # Keyboard event handling and simulation
│   ├── lib.rs           # Library exports
│   ├── main.rs          # CLI entry point
│   ├── models.rs        # Data models
│   ├── storage.rs       # Persistent storage operations
│   └── ui.rs            # Terminal user interface for snippet management
```

## Core Components

### 1. Configuration System

`config.rs` handles all configuration-related operations, including:
- Locating and creating the configuration directory
- Managing paths to database and PID files
- Detecting daemon status

### 2. Daemon Process

`daemon.rs` implements the background daemon that:
- Monitors keyboard input
- Detects trigger sequences (`:shortcut`)
- Expands text via keyboard simulation
- Monitors for database changes
- Handles process management (start/stop)

### 3. Text Expansion

`expansion.rs` contains the core expansion logic:
- Parsing input buffers for expansion triggers
- Matching shortcuts against the snippet database
- Managing text replacement via keyboard events

### 4. Data Model

`models.rs` defines the data structures:
- `SnippetEntry`: Represents a text snippet with shortcut, content, and metadata

### 5. Storage

`storage.rs` handles persistence:
- Loading snippets from JSON database
- Saving snippets to disk
- CRUD operations (Create, Read, Update, Delete)

### 6. UI Components

- `ui.rs`: Main terminal UI for browsing and managing snippets
- `interactive.rs`: Interactive UI for adding new snippets

### 7. Keyboard Processing

`keyboard.rs` provides:
- Keyboard event listening
- Character detection and conversion
- Keyboard simulation for text replacement

### 8. Error Handling

`error.rs` implements a unified error system using Rust's error trait.

## Data Flow

1. **Input Detection:**
   - The daemon listens for keyboard events
   - Characters are accumulated in a buffer
   - When a trigger character (space, tab, etc.) is detected, the buffer is processed

2. **Text Expansion:**
   - If the buffer starts with `:` followed by a registered shortcut
   - The daemon deletes the trigger sequence
   - The expanded text is simulated as keyboard input

3. **Snippet Management:**
   - CLI commands and UI operations modify the snippet database
   - Changes are persisted to disk
   - The daemon monitors for changes and reloads as needed

## Design Decisions

### Daemon Architecture

- The daemon runs as a background process to minimize resource usage
- On Unix systems, it forks to the background
- On non-Unix systems, it runs in the foreground

### Expansion Method

- Using keyboard simulation ensures compatibility across applications
- The approach works in all text fields that accept keyboard input

### Persistent Storage

- JSON format for human readability and easy manipulation
- Single file storage for simplicity

### Terminal UI

- Built with ratatui and crossterm for cross-platform compatibility
- Provides a modern, interactive interface without GUI dependencies
