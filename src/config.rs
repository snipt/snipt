use crate::error::{Result, ScribeError};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const SPECIAL_CHAR: char = ':';
pub const PID_FILENAME: &str = "scribe-daemon.pid";
pub const DB_FILENAME: &str = "scribe.json";

/// Get the scribe configuration directory
pub fn get_config_dir() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"))
}

/// Ensure the configuration directory exists
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let db_path = get_db_file_path();
    if !db_path.exists() {
        create_empty_file(&db_path, "database file")?;
    }

    Ok(config_dir)
}

/// Create an empty config file at the specified path
pub fn create_empty_file(path: &PathBuf, description: &str) -> Result<()> {
    println!("Creating {} at: {}", description, path.display());
    fs::write(path, "")?;
    Ok(())
}

/// Get the path to the PID file
pub fn get_pid_file_path() -> PathBuf {
    get_config_dir().join(PID_FILENAME)
}

/// Get the path to the database file
pub fn get_db_file_path() -> PathBuf {
    get_config_dir().join(DB_FILENAME)
}

/// Check if the database file exists
pub fn db_file_exists() -> bool {
    get_db_file_path().exists()
}

/// Check if daemon is running
pub fn is_daemon_running() -> Result<Option<u32>> {
    let pid_file = get_pid_file_path();

    if !pid_file.exists() {
        return Ok(None);
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid = pid_str
        .trim()
        .parse::<u32>()
        .map_err(|_| ScribeError::InvalidPid)?;

    #[cfg(unix)]
    {
        let status = std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status();

        if status.is_ok() && status.unwrap().success() {
            return Ok(Some(pid));
        }
        return Ok(None);
    }

    // For non-Unix systems, assume it's running if PID file exists
    #[cfg(not(unix))]
    {
        Ok(Some(pid))
    }
}
