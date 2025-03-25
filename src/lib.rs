//! Scribe - A text snippet expansion tool.
//!
//! Scribe allows you to define text snippets with shortcuts and expand them
//! as you type using a special character followed by the shortcut.

pub mod config;
pub mod daemon;
pub mod error;
pub mod expansion;
pub mod keyboard;
pub mod models;
pub mod storage;
pub mod ui;

// Re-export the most commonly used types and functions
pub use config::{get_config_dir, SPECIAL_CHAR};
pub use daemon::{daemon_status, run_daemon_worker, start_daemon, stop_daemon};
pub use error::{Result, ScribeError};
pub use models::SnippetEntry;
pub use storage::{add_snippet, delete_snippet, load_snippets, update_snippet};
pub use ui::display_snippet_manager;
