use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ScribeError {
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
}

impl fmt::Display for ScribeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScribeError::Io(err) => write!(f, "I/O error: {}", err),
            ScribeError::Json(err) => write!(f, "JSON error: {}", err),
            ScribeError::Enigo(err) => write!(f, "Keyboard controller error: {}", err),
            ScribeError::Keyboard(err) => write!(f, "Keyboard error: {}", err),
            ScribeError::DatabaseNotFound(path) => write!(f, "Database not found at: {}", path),
            ScribeError::DaemonAlreadyRunning(pid) => {
                write!(f, "Daemon already running with PID {}", pid)
            }
            ScribeError::DaemonNotRunning => write!(f, "Daemon is not running"),
            ScribeError::InvalidPid => write!(f, "Invalid PID in daemon file"),
            ScribeError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            ScribeError::Clipboard(msg) => write!(f, "Clipboard error: {}", msg),
            ScribeError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ScribeError {}

impl From<io::Error> for ScribeError {
    fn from(err: io::Error) -> Self {
        ScribeError::Io(err)
    }
}

impl From<serde_json::Error> for ScribeError {
    fn from(err: serde_json::Error) -> Self {
        ScribeError::Json(err)
    }
}

pub type Result<T> = std::result::Result<T, ScribeError>;
