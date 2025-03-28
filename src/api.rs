use crate::config::{get_db_file_path, is_daemon_running};
use crate::models::SnippetEntry;
use crate::storage::{add_snippet, delete_snippet, load_snippets, update_snippet};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// Get all snippets
pub fn api_get_snippets() -> ApiResponse<Vec<SnippetEntry>> {
    match load_snippets() {
        Ok(snippets) => ApiResponse::success(snippets),
        Err(e) => ApiResponse::error(format!("Failed to load snippets: {}", e)),
    }
}

// Get a specific snippet
pub fn api_get_snippet(shortcut: &str) -> ApiResponse<Option<SnippetEntry>> {
    match load_snippets() {
        Ok(snippets) => {
            let found = snippets.iter().find(|s| s.shortcut == shortcut).cloned();
            ApiResponse::success(found)
        }
        Err(e) => ApiResponse::error(format!("Failed to load snippets: {}", e)),
    }
}

// Add a new snippet
pub fn api_add_snippet(shortcut: String, snippet: String) -> ApiResponse<()> {
    match add_snippet(shortcut, snippet) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to add snippet: {}", e)),
    }
}

// Update an existing snippet
pub fn api_update_snippet(shortcut: String, snippet: String) -> ApiResponse<()> {
    match update_snippet(&shortcut, snippet) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to update snippet: {}", e)),
    }
}

// Delete a snippet
pub fn api_delete_snippet(shortcut: String) -> ApiResponse<()> {
    match delete_snippet(&shortcut) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to delete snippet: {}", e)),
    }
}

// Get daemon status
pub fn api_daemon_status() -> ApiResponse<bool> {
    match is_daemon_running() {
        Ok(status) => ApiResponse::success(status.is_some()),
        Err(e) => ApiResponse::error(format!("Failed to check daemon status: {}", e)),
    }
}

// Get daemon details
pub fn api_daemon_details() -> ApiResponse<DaemonStatus> {
    match is_daemon_running() {
        Ok(status) => {
            let status = DaemonStatus {
                running: status.is_some(),
                pid: status,
                config_path: get_db_file_path().to_string_lossy().to_string(),
            };
            ApiResponse::success(status)
        }
        Err(e) => ApiResponse::error(format!("Failed to check daemon status: {}", e)),
    }
}

#[derive(Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub config_path: String,
}
