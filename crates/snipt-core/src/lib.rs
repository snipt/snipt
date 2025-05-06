pub mod config;
pub mod error;
pub mod execution;
pub mod expansion;
pub mod keyboard;
pub mod models;
pub mod storage;

// Re-export common items for convenience
pub use config::{get_config_dir, is_daemon_running, EXECUTE_CHAR, SPECIAL_CHAR};
pub use error::{Result, SniptError};
pub use execution::is_url;
pub use expansion::{determine_expansion_style, handle_expansion, ExpansionStyle, ExpansionType};
pub use models::SnippetEntry;
pub use storage::{add_snippet, delete_snippet, load_snippets, update_snippet};
