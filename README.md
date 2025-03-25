# Scribe: Text Snippet Expansion Tool

Scribe is a versatile text snippet expansion tool that lets you define and use text shortcuts across your system. Type a special character followed by your shortcut, and Scribe automatically expands it into your predefined text.

## Features

- Define custom text snippets with shortcut aliases
- Background daemon that monitors your typing and expands shortcuts
- Terminal-based UI for managing snippets
- Interactive mode for adding new snippets
- Cross-platform support (Linux, macOS, Windows)
- Real-time clipboard integration

## Installation

### From Source

1. Ensure you have Rust and Cargo installed. If not, install from [rustup.rs](https://rustup.rs/).

2. Clone the repository and build:

```bash
git clone https://github.com/yourusername/scribe.git
cd scribe
cargo build --release
```

3. Install the binary (optional):

```bash
cargo install --path .
```

## Usage

### Basic Commands

```bash
# Start the daemon (must be running for text expansion)
scribe start

# Stop the daemon
scribe stop

# Check daemon status
scribe status

# Add a new snippet
scribe add --shortcut hello --snippet "Hello, world!"

# Add a snippet interactively
scribe new

# List all snippets in the terminal UI
scribe list

# Delete a snippet
scribe delete --shortcut hello

# Update an existing snippet
scribe update --shortcut hello --snippet "Hello there, world!"
```

### Using Snippets

Once you've added snippets and started the daemon, you can use them by typing:

```
:shortcut
```

For example, `:hello` would expand to "Hello, world!" based on the example above.

## Terminal UI

Scribe includes a terminal-based UI for managing your snippets:

- View all snippets with their shortcuts
- Search and filter snippets
- Copy snippets to clipboard
- Delete snippets
- View help and usage tips

Launch the UI with:

```bash
scribe list
```

### UI Navigation

- ↑/↓: Navigate through snippets
- Tab: Switch between tabs
- Enter: Copy selected snippet to clipboard
- /: Search snippets
- Ctrl+D: Delete current snippet
- Esc/q: Exit

## Configuration

Scribe stores its configuration and snippet database in the `~/.scribe` directory:

- `scribe.json`: Contains your snippet database
- `scribe-daemon.pid`: Contains the process ID of the running daemon

## Requirements

- Rust 1.56 or later
- Dependencies (automatically managed by Cargo):
  - rdev (for keyboard event handling)
  - clap (for command-line parsing)
  - serde (for JSON serialization)
  - crossterm (for terminal UI)
  - ratatui (for terminal UI)
  - enigo (for keyboard simulation)
  - arboard (for clipboard access)

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
