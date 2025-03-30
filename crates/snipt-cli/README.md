# Snipt CLI

This crate provides the command-line interface for snipt, a text snippet expansion tool. It handles user commands for managing snippets, controlling the daemon, and launching the UI.

## Features

- **Command-Line Interface**: Parse and execute user commands for managing snippets
- **Interactive UI**: Launch the TUI (Terminal User Interface) for snippet management
- **Daemon Control**: Start, stop, and check the status of the snipt daemon
- **API Server Management**: Control the HTTP API server for the Electron UI

## CLI Commands

- `snipt`: Launch the interactive TUI where you can manage all snippets and settings
- `snipt add`: Add a new text snippet
- `snipt delete`: Delete a text snippet
- `snipt update`: Update an existing snippet
- `snipt new`: Add a new snippet interactively
- `snipt start`: Start the daemon and API server
- `snipt stop`: Stop the snipt daemon
- `snipt status`: Check daemon status
- `snipt list`: Display all snippets in a TUI
- `snipt serve`: Start just the API server
- `snipt port`: Show the API server port
- `snipt api-status`: Check API server health
- `snipt api-diagnose`: Diagnose API server issues

## Architecture

This crate integrates with other snipt components:

- `snipt-core`: Core functionality for snippet management
- `snipt-daemon`: Daemon process that listens for keyboard events
- `snipt-server`: HTTP API server for the Electron UI
- `snipt-ui`: Terminal user interface components

## Development

Use `cargo run` to start the CLI in development mode. For usage examples, run `cargo run -- --help`.
