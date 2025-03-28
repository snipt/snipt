pub mod config;
pub mod error;
pub mod expansion;
pub mod keyboard;
pub mod models;
pub mod storage;

// Re-export common items for convenience
pub use config::{get_config_dir, is_daemon_running, SPECIAL_CHAR};
pub use error::{Result, ScribeError};
pub use models::SnippetEntry;
pub use storage::{add_snippet, delete_snippet, load_snippets, update_snippet};
