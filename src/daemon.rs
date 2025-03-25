use crate::config::{
    db_file_exists, ensure_config_dir, get_db_file_path, get_pid_file_path, is_daemon_running,
};
use crate::error::{Result, ScribeError};
use crate::expansion::{process_expansion, replace_text};
use crate::keyboard::rdev_key_to_char;
use crate::storage::load_snippets;

use rdev::{self, listen, EventType, Key as RdevKey};
use std::fs::{self, File};
use std::io::Write;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Start the daemon process
pub fn start_daemon() -> Result<()> {
    // Check if daemon is already running
    if let Some(pid) = is_daemon_running()? {
        return Err(ScribeError::DaemonAlreadyRunning(pid));
    }

    // Ensure config directory exists
    ensure_config_dir()?;

    // Check if database exists
    if !db_file_exists() {
        return Err(ScribeError::DatabaseNotFound(
            get_db_file_path().to_string_lossy().to_string(),
        ));
    }

    // Fork to background on Unix systems
    #[cfg(unix)]
    {
        use daemonize::Daemonize;
        println!("Starting scribe daemon in the background");

        let pid_file = get_pid_file_path();

        // Create a new daemonize process
        let daemonize = Daemonize::new()
            .pid_file(&pid_file)
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(File::create("/dev/null")?)
            .stderr(File::create("/dev/null")?);

        match daemonize.start() {
            Ok(_) => run_daemon_worker(), // We're now in the daemon process
            Err(e) => {
                let msg = format!("Error starting daemon: {}", e);
                Err(ScribeError::Other(msg))
            }
        }
    }

    // For non-Unix systems, just continue execution
    #[cfg(not(unix))]
    {
        println!("Starting scribe daemon in the foreground (background not supported on this OS)");
        run_daemon_worker()
    }
}

/// The actual daemon worker process
pub fn run_daemon_worker() -> Result<()> {
    // Create PID file
    let pid_file = get_pid_file_path();
    let mut file = File::create(&pid_file)?;
    write!(file, "{}", process::id())?;

    // Load snippets
    let db_path = get_db_file_path();
    if !db_path.exists() {
        return Err(ScribeError::DatabaseNotFound(
            db_path.to_string_lossy().to_string(),
        ));
    }

    // Load the scribe database
    let snippets = Arc::new(Mutex::new(load_snippets()?));

    // Track running state
    let running = Arc::new(Mutex::new(true));
    let running_clone = Arc::clone(&running);

    // Buffer for text accumulation
    let text_buffer = Arc::new(Mutex::new(String::new()));
    let snippets_clone = Arc::clone(&snippets);
    let buffer_clone = Arc::clone(&text_buffer);

    // Start keyboard event listener in a separate thread
    let keyboard_thread = thread::spawn(move || {
        if let Err(e) = listen(move |event| {
            if !*running_clone.lock().unwrap() {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => {
                    let mut buffer = buffer_clone.lock().unwrap();

                    match key {
                        RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                            // Check if we should expand the current buffer
                            if !buffer.is_empty() {
                                let snippets_guard = snippets_clone.lock().unwrap();
                                if let Ok(Some(expansion)) =
                                    process_expansion(&buffer, &snippets_guard)
                                {
                                    // Delete the special character and shortcut, then type the expanded text
                                    let _ = replace_text(buffer.len() + 1, &expansion);
                                }
                            }

                            // Clear buffer regardless of expansion
                            buffer.clear();
                        }
                        RdevKey::Backspace => {
                            if !buffer.is_empty() {
                                buffer.pop();
                            }
                        }
                        _ => {
                            // Add the character to our buffer
                            if let Some(c) = rdev_key_to_char(&key, &event) {
                                buffer.push(c);

                                // Limit buffer size to prevent memory issues
                                if buffer.len() > 100 {
                                    buffer.remove(0);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }) {
            eprintln!("Error setting up keyboard listener: {:?}", e);
        }
    });

    // Monitor for termination signals or other conditions to stop the daemon
    let check_interval = Duration::from_secs(1);
    while *running.lock().unwrap() {
        thread::sleep(check_interval);
    }

    // Wait for keyboard thread to finish
    if let Err(e) = keyboard_thread.join() {
        eprintln!("Error joining keyboard thread: {:?}", e);
    }

    // Cleanup
    if let Err(e) = fs::remove_file(&pid_file) {
        eprintln!("Error removing PID file: {}", e);
    }

    Ok(())
}

/// Stop the daemon if it's running
pub fn stop_daemon() -> Result<()> {
    let pid_file = get_pid_file_path();

    if !pid_file.exists() {
        return Err(ScribeError::DaemonNotRunning);
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid = pid_str
        .trim()
        .parse::<u32>()
        .map_err(|_| ScribeError::InvalidPid)?;

    #[cfg(unix)]
    {
        let status = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();

        if let Ok(status) = status {
            if status.success() {
                println!("Stopped scribe daemon with PID {}", pid);
                fs::remove_file(&pid_file)?;
                return Ok(());
            }
        }

        return Err(ScribeError::Other(format!(
            "Failed to stop daemon with PID {}",
            pid
        )));
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let status = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .status();

        if let Ok(status) = status {
            if status.success() {
                println!("Stopped scribe daemon with PID {}", pid);
                fs::remove_file(&pid_file)?;
                return Ok(());
            }
        }

        return Err(ScribeError::Other(format!(
            "Failed to stop daemon with PID {}",
            pid
        )));
    }

    #[cfg(not(any(unix, windows)))]
    {
        return Err(ScribeError::Other(
            "Stopping daemon not supported on this platform".to_string(),
        ));
    }
}

/// Check daemon status
pub fn daemon_status() -> Result<()> {
    match is_daemon_running()? {
        Some(pid) => {
            println!("scribe daemon is running with PID {}", pid);
            Ok(())
        }
        None => {
            println!("scribe daemon is not running");
            Ok(())
        }
    }
}
