
# Scribe: Text Snippet Expansion Tool

<div align="center">

![Scribe Logo](https://via.placeholder.com/150x150)

**Type less, say more. Text expansion that works everywhere.**

[![GitHub license](https://img.shields.io/github/license/snipt/scribe)](https://github.com/snipt/scribe/blob/main/LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.56+-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue.svg)]()

</div>

---
## 🚀 Overview

Scribe is a powerful text snippet expansion tool that boosts your productivity by replacing short text shortcuts with longer content. Just type a prefix (like `:`) followed by your shortcut, and Scribe automatically expands it into your predefined text.

## ✨ Key Features

- **Custom Text Snippets**: Define shortcut aliases for frequently used text
- **System-Wide Expansion**: Works in any application where you can type
- **Modern TUI**: Beautiful terminal interface for managing snippets
- **Background Daemon**: Silent monitoring of keyboard input for expansion
- **Cross-Platform**: Works seamlessly on Linux, macOS, and Windows
- **Clipboard Integration**: Quickly copy expansions to clipboard

## 📦 Installation

### From Source

```bash
# 1. Install Rust if needed (https://rustup.rs/)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone and build Scribe
git clone https://github.com/bahdotsh/scribe.git
cd scribe
cargo build --release

# 3. Install the binary (optional)
cargo install --path .
```

## 🎮 Usage

### Starting Scribe

```bash
# Launch interactive UI
scribe

# Start the daemon (required for expansion)
scribe start
```

### Managing Snippets

```bash
# Add a snippet
scribe add --shortcut hello --snippet "Hello, world!"

# Add interactively
scribe new

# View and manage all snippets
scribe list

# Remove a snippet
scribe delete --shortcut hello

# Update existing snippet
scribe update --shortcut hello --snippet "Hello there, world!"
```

### Monitoring & Control

```bash
# Check daemon status
scribe status

# Stop the daemon
scribe stop
```

## 💡 How Expansion Works

Once the daemon is running, type your prefix followed by a shortcut anywhere on your system:

```
:hello
```

This instantly expands to "Hello, world!" (or your custom text).

## 🖥️ Terminal User Interface

<div align="center">

![Scribe TUI Screenshot](https://via.placeholder.com/600x400)

</div>

Launch the beautiful terminal UI with either `scribe` or `scribe list`.

### Navigation

| Key         | Action                     |
|-------------|----------------------------|
| ↑/↓         | Navigate through snippets  |
| Tab         | Switch between tabs        |
| Enter       | Copy to clipboard          |
| /           | Search snippets            |
| Ctrl+D      | Delete selected snippet    |
| Esc/q       | Exit                       |

## ⚙️ Configuration

Scribe stores your data in `~/.scribe/`:

- `scribe.json`: Your snippet database
- `scribe-daemon.pid`: Process ID of running daemon

## 🧩 Architecture

Scribe consists of several components:

- **Core Library**: Handles snippet management and persistence
- **Daemon**: Background process that listens for keyboard events
- **CLI**: Command-line interface for controlling Scribe
- **TUI**: Terminal user interface for snippet management
- **Server**: HTTP API for potential GUI clients

## 🔨 Requirements

- Rust 1.56+
- Core dependencies: rdev, clap, serde, crossterm, ratatui, enigo, arboard

## 🤝 Contributing

Contributions are welcome! Please feel free to:

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

## 📜 License

MIT

---

<div align="center">
  <p>Built with ❤️ using Rust</p>
</div>
