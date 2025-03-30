use std::fmt;
use std::io;

#[derive(Debug)]
pub enum SniptError {
    Io(io::Error),
    Json(serde_json::Error),
    Enigo(String), // Changed from specific Error types
    Keyboard(String),
    DatabaseNotFound(String),
    DaemonAlreadyRunning(u32),
    DaemonNotRunning,
    InvalidPid,
    InvalidConfig(String),
    Clipboard(String),
    Other(String),
    PermissionDenied(String),
}

impl fmt::Display for SniptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SniptError::Io(err) => write!(f, "I/O error: {}", err),
            SniptError::Json(err) => write!(f, "JSON error: {}", err),
            SniptError::Enigo(err) => write!(f, "Keyboard controller error: {}", err),
            SniptError::Keyboard(err) => write!(f, "Keyboard error: {}", err),
            SniptError::DatabaseNotFound(path) => write!(f, "Database not found at: {}", path),
            SniptError::DaemonAlreadyRunning(pid) => {
                write!(f, "Daemon already running with PID {}", pid)
            }
            SniptError::DaemonNotRunning => write!(f, "Daemon is not running"),
            SniptError::InvalidPid => write!(f, "Invalid PID in daemon file"),
            SniptError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            SniptError::Clipboard(msg) => write!(f, "Clipboard error: {}", msg),
            SniptError::Other(msg) => write!(f, "Error: {}", msg),
            SniptError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for SniptError {}

impl From<io::Error> for SniptError {
    fn from(err: io::Error) -> Self {
        SniptError::Io(err)
    }
}

impl From<serde_json::Error> for SniptError {
    fn from(err: serde_json::Error) -> Self {
        SniptError::Json(err)
    }
}

pub type Result<T> = std::result::Result<T, SniptError>;
