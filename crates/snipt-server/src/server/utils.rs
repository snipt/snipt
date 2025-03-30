//! Utilities for managing the API server.

use snipt_core::{get_config_dir, SniptError, Result};
use std::fs;
use std::io::{Read, Write};

/// Try to get the API server port from stored configuration
pub fn get_api_server_port() -> Result<u16> {
    let port_file_path = get_config_dir().join("api_port.txt");

    if port_file_path.exists() {
        let mut file = fs::File::open(port_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        contents
            .trim()
            .parse::<u16>()
            .map_err(|_| SniptError::Other("Invalid port stored in configuration".to_string()))
    } else {
        Err(SniptError::Other(
            "API server port information not found".to_string(),
        ))
    }
}

/// Check if a port is available by trying to bind to it
pub fn port_is_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Save the API port to a configuration file
pub fn save_api_port(port: u16) -> Result<()> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let port_file_path = config_dir.join("api_port.txt");
    let mut file = fs::File::create(port_file_path)?;
    write!(file, "{}", port)?;

    Ok(())
}

/// Test if a port is available asynchronously
pub async fn test_port_availability(port: u16) -> bool {
    use std::net::TcpListener;

    // Try to bind to the port to see if it's available
    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => true,   // Port is available
        Err(_) => false, // Port is in use or cannot be bound to
    }
}
