//! Scribe - A text snippet expansion tool.
//!
//! Scribe allows you to define text snippets with shortcuts and expand them
//! as you type using a special character followed by the shortcut.

pub mod api;
pub mod config;
pub mod daemon;
pub mod error;
pub mod expansion;
pub mod interactive;
pub mod keyboard;
pub mod models;
pub mod server;
pub mod storage;
pub mod ui;

// Re-export
pub use api::{ApiResponse, DaemonStatus};
pub use config::{get_config_dir, is_daemon_running, SPECIAL_CHAR};
pub use daemon::{daemon_status, run_daemon_worker, start_daemon, stop_daemon};
pub use error::{Result, ScribeError};
pub use interactive::interactive_add;
pub use interactive::AddResult;
pub use models::SnippetEntry;
pub use server::start_api_server;
pub use storage::{add_snippet, delete_snippet, load_snippets, update_snippet};
pub use ui::{display_scribe_dashboard, display_snippet_manager};
