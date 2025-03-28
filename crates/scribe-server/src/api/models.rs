//! Data models for API requests and responses.

use serde::{Deserialize, Serialize};

/// Standard API response format
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

/// Daemon status information
#[derive(Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub config_path: String,
    pub api_server: ApiServerInfo,
}

/// API server information
#[derive(Serialize, Deserialize)]
pub struct ApiServerInfo {
    pub port: u16,
    pub url: String,
}

/// Request model for adding or updating a snippet
#[derive(Deserialize)]
pub struct SnippetRequest {
    pub shortcut: String,
    pub snippet: String,
}

/// Request model for retrieving a single snippet
#[derive(Deserialize)]
pub struct GetSnippetRequest {
    pub shortcut: String,
}

/// Request model for deleting a snippet
#[derive(Deserialize)]
pub struct DeleteSnippetRequest {
    pub shortcut: String,
}
