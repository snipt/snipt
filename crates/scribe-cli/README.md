# Scribe CLI

This crate provides the command-line interface for Scribe, a text snippet expansion tool. It handles user commands for managing snippets, controlling the daemon, and launching the UI.

## Features

- **Command-Line Interface**: Parse and execute user commands for managing snippets
- **Interactive UI**: Launch the TUI (Terminal User Interface) for snippet management
- **Daemon Control**: Start, stop, and check the status of the Scribe daemon
- **API Server Management**: Control the HTTP API server for the Electron UI

## CLI Commands

- `scribe`: Launch the interactive TUI where you can manage all snippets and settings
- `scribe add`: Add a new text snippet
- `scribe delete`: Delete a text snippet
- `scribe update`: Update an existing snippet
- `scribe new`: Add a new snippet interactively
- `scribe start`: Start the daemon and API server
- `scribe stop`: Stop the scribe daemon
- `scribe status`: Check daemon status
- `scribe list`: Display all snippets in a TUI
- `scribe serve`: Start just the API server
- `scribe port`: Show the API server port
- `scribe api-status`: Check API server health
- `scribe api-diagnose`: Diagnose API server issues

## Architecture

This crate integrates with other Scribe components:

- `scribe-core`: Core functionality for snippet management
- `scribe-daemon`: Daemon process that listens for keyboard events
- `scribe-server`: HTTP API server for the Electron UI
- `scribe-ui`: Terminal user interface components

## Development

Use `cargo run` to start the CLI in development mode. For usage examples, run `cargo run -- --help`.
