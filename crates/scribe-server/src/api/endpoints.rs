use crate::{
    api::models::{ApiResponse, ApiServerInfo, DaemonStatus},
    server::utils::get_api_server_port,
};
use scribe_core::{
    add_snippet, config::get_db_file_path, delete_snippet, is_daemon_running, load_snippets,
    update_snippet, SnippetEntry,
};

/// Get all snippets
pub fn get_snippets() -> ApiResponse<Vec<SnippetEntry>> {
    match load_snippets() {
        Ok(snippets) => ApiResponse::success(snippets),
        Err(e) => ApiResponse::error(format!("Failed to load snippets: {}", e)),
    }
}

/// Get a specific snippet by shortcut
pub fn get_snippet(shortcut: &str) -> ApiResponse<Option<SnippetEntry>> {
    match load_snippets() {
        Ok(snippets) => {
            let found = snippets.iter().find(|s| s.shortcut == shortcut).cloned();
            ApiResponse::success(found)
        }
        Err(e) => ApiResponse::error(format!("Failed to load snippets: {}", e)),
    }
}

/// Add a new snippet
pub fn add_snippet_handler(shortcut: String, snippet: String) -> ApiResponse<()> {
    match add_snippet(shortcut, snippet) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to add snippet: {}", e)),
    }
}

/// Update an existing snippet
pub fn update_snippet_handler(shortcut: String, snippet: String) -> ApiResponse<()> {
    match update_snippet(&shortcut, snippet) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to update snippet: {}", e)),
    }
}

/// Delete a snippet
pub fn delete_snippet_handler(shortcut: String) -> ApiResponse<()> {
    match delete_snippet(&shortcut) {
        Ok(_) => ApiResponse::success(()),
        Err(e) => ApiResponse::error(format!("Failed to delete snippet: {}", e)),
    }
}

/// Get daemon running status
pub fn get_daemon_status() -> ApiResponse<bool> {
    match is_daemon_running() {
        Ok(status) => ApiResponse::success(status.is_some()),
        Err(e) => ApiResponse::error(format!("Failed to check daemon status: {}", e)),
    }
}

/// Get detailed daemon information
pub fn get_daemon_details(port: u16) -> ApiResponse<DaemonStatus> {
    match is_daemon_running() {
        Ok(status) => {
            // Try to get the actual port in case it's different
            let actual_port = get_api_server_port().unwrap_or(port);

            let status = DaemonStatus {
                running: status.is_some(),
                pid: status,
                config_path: get_db_file_path().to_string_lossy().to_string(),
                api_server: ApiServerInfo {
                    port: actual_port,
                    url: format!("http://localhost:{}", actual_port),
                },
            };
            ApiResponse::success(status)
        }
        Err(e) => ApiResponse::error(format!("Failed to check daemon status: {}", e)),
    }
}
