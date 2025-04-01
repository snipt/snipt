use crate::error::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

pub const SPECIAL_CHAR: char = ':';
pub const PID_FILENAME: &str = "snipt-daemon.pid";
pub const DB_FILENAME: &str = "snipt.json";
pub const EXECUTE_CHAR: char = '!';

/// Get the snipt configuration directory
pub fn get_config_dir() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".snipt"))
        .unwrap_or_else(|_| PathBuf::from(".snipt"))
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

    if pid_file.exists() {
        match fs::read_to_string(&pid_file) {
            Ok(contents) => {
                match contents.trim().parse::<u32>() {
                    Ok(pid) => Ok(Some(pid)),
                    Err(_) => {
                        // Invalid PID, treat as not running and clean up
                        let _ = fs::remove_file(&pid_file);
                        Ok(None)
                    }
                }
            }
            Err(_) => {
                // Can't read file, treat as not running and clean up
                let _ = fs::remove_file(&pid_file);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
