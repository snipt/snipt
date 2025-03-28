mod common;
mod dashboard;
mod editor;
mod snippet_manager;

// Public API
pub use dashboard::display_scribe_dashboard;
pub use editor::{interactive_add, AddResult};
pub use snippet_manager::display_snippet_manager;
