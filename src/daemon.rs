use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use rdev::{self, EventType, Key as RdevKey, listen};
use serde_json::Value;

const SPECIAL_CHAR: char = ':';

pub fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // Check if daemon is already running
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    if !pid_dir.exists() {
        fs::create_dir_all(&pid_dir)?;
    }

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if pid_file.exists() {
        let pid_str = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(unix)]
            {
                let status = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status();

                if status.is_ok() && status.unwrap().success() {
                    println!("scribe daemon is already running with PID {}", pid);
                    return Ok(());
                }
            }

            #[cfg(not(unix))]
            {
                println!("Cannot verify if daemon is running, assuming it's not");
            }
        }
    }

    // Fork to background on Unix systems
    #[cfg(unix)]
    {
        use daemonize::Daemonize;
        println!("Starting scribe daemon in the background");

        let pid_dir = env::var("HOME")
            .map(|home| PathBuf::from(home).join(".scribe"))
            .unwrap_or_else(|_| PathBuf::from(".scribe"));

        let pid_file = pid_dir.join("scribe-daemon.pid");

        // Create a new daemonize process
        let daemonize = Daemonize::new()
            .pid_file(&pid_file)
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(std::fs::File::create("/dev/null")?)
            .stderr(std::fs::File::create("/dev/null")?);

        match daemonize.start() {
            Ok(_) => {
                // We're now in the daemon process
                run_daemon_worker()
            }
            Err(e) => {
                eprintln!("Error starting daemon: {}", e);
                Err(e.into())
            }
        }
    }

    // For non-Unix systems, just continue execution
    #[cfg(not(unix))]
    {
        println!("Starting scribe daemon in the foreground (background not supported on this OS)");
        return run_daemon_worker();
    }
}

// The actual daemon worker process
pub fn run_daemon_worker() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");
    let mut file = File::create(&pid_file)?;
    write!(file, "{}", process::id())?;

    let scribe_db_path = pid_dir.join("scribe.json");

    if !scribe_db_path.exists() {
        println!("Scribe database not found at: {:?}", scribe_db_path);
        return Ok(());
    }

    // Load the scribe database
    let scribe_db = Arc::new(Mutex::new(load_scribe_db(&scribe_db_path)?));

    // Track if the daemon should continue running
    let running = Arc::new(Mutex::new(true));
    let running_clone = Arc::clone(&running);

    // Data for tracking potential shortcuts
    let current_text = Arc::new(Mutex::new(String::new()));
    let scribe_db_clone = Arc::clone(&scribe_db);
    let current_text_clone = Arc::clone(&current_text);

    // Start keyboard event listener in a separate thread
    let keyboard_thread = thread::spawn(move || {
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to initialize Enigo: {}", e);
                return;
            }
        };

        if let Err(e) = listen(move |event| {
            if !*running_clone.lock().unwrap() {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => {
                    let mut current = current_text_clone.lock().unwrap();

                    match key {
                        RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                            // Check if the current text is a shortcut that starts with SPECIAL_CHAR
                            if current.starts_with(SPECIAL_CHAR) && current.len() > 1 {
                                let shortcut = current.trim_start_matches(SPECIAL_CHAR);

                                // Access the database and look for the shortcut
                                let db = scribe_db_clone.lock().unwrap();
                                if let Some(snippet) = get_snippet(&db, shortcut) {
                                    if let Some(snippet_text) = snippet.as_str() {
                                        // Calculate total length to delete - this includes the special character
                                        let chars_to_delete = current.len() + 1; // This includes both the special char and the shortcut text

                                        // Backspace to remove everything (special char + shortcut)
                                        for _ in 0..chars_to_delete {
                                            // Add a small delay between backspaces to ensure they're registered
                                            thread::sleep(Duration::from_millis(5));
                                            if let Err(e) =
                                                enigo.key(Key::Backspace, Direction::Click)
                                            {
                                                eprintln!("Error sending backspace: {}", e);
                                            }
                                        }

                                        // Small delay before typing the replacement text
                                        thread::sleep(Duration::from_millis(10));

                                        // Type the expanded snippet
                                        if let Err(e) = enigo.text(snippet_text) {
                                            eprintln!("Error typing snippet: {}", e);
                                        }
                                    }
                                }
                            }

                            // Clear the buffer after expansion/processing
                            current.clear();
                        }
                        RdevKey::Backspace => {
                            if !current.is_empty() {
                                current.pop();
                            }
                        }
                        _ => {
                            // Add the character to our current buffer
                            if let Some(c) = rdev_key_to_char(&key, &event) {
                                current.push(c);
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

    // Clean up and wait for keyboard thread to finish
    if let Err(e) = keyboard_thread.join() {
        eprintln!("Error joining keyboard thread: {:?}", e);
    }

    // Cleanup
    if let Err(e) = fs::remove_file(&pid_file) {
        eprintln!("Error removing PID file: {}", e);
    }

    Ok(())
}

// Helper function to convert rdev::Key to char
fn rdev_key_to_char(key: &RdevKey, event: &rdev::Event) -> Option<char> {
    // Map special keys to characters
    match key {
        // Use proper casing for key variants
        RdevKey::Kp0 if event.name == Some("!".to_string()) => return Some('!'),
        RdevKey::Kp1 if event.name == Some("@".to_string()) => return Some('@'),
        RdevKey::Kp2 if event.name == Some("#".to_string()) => return Some('#'),
        RdevKey::Kp3 if event.name == Some("$".to_string()) => return Some('$'),
        RdevKey::Kp4 if event.name == Some("%".to_string()) => return Some('%'),
        RdevKey::Kp5 if event.name == Some("^".to_string()) => return Some('^'),
        RdevKey::Kp6 if event.name == Some("&".to_string()) => return Some('&'),
        RdevKey::Kp7 if event.name == Some("*".to_string()) => return Some('*'),
        RdevKey::Kp8 if event.name == Some("(".to_string()) => return Some('('),
        RdevKey::Kp9 if event.name == Some(")".to_string()) => return Some(')'),
        RdevKey::KpMinus if event.name == Some("_".to_string()) => return Some('_'),
        RdevKey::Equal if event.name == Some("+".to_string()) => return Some('+'),
        RdevKey::SemiColon if event.name == Some(":".to_string()) => return Some(':'),
        RdevKey::SemiColon if event.name == Some(";".to_string()) => return Some(';'),
        RdevKey::Quote if event.name == Some("\"".to_string()) => return Some('"'),
        RdevKey::Quote if event.name == Some("'".to_string()) => return Some('\''),
        RdevKey::Comma if event.name == Some("<".to_string()) => return Some('<'),
        RdevKey::Comma if event.name == Some(",".to_string()) => return Some(','),
        RdevKey::Dot if event.name == Some(">".to_string()) => return Some('>'),
        RdevKey::Dot if event.name == Some(".".to_string()) => return Some('.'),
        RdevKey::Slash if event.name == Some("?".to_string()) => return Some('?'),
        RdevKey::Slash if event.name == Some("/".to_string()) => return Some('/'),
        RdevKey::BackSlash if event.name == Some("|".to_string()) => return Some('|'),
        RdevKey::BackSlash if event.name == Some("\\".to_string()) => return Some('\\'),
        // Add more special keys as needed
        _ => {}
    }

    // For regular alphabetic keys, use the name
    if let Some(name) = &event.name {
        // Handle simple single character keys
        if name.len() == 1 {
            return name.chars().next();
        }
    }

    None
}

fn load_scribe_db(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.into())
}

fn get_snippet<'a>(scribe_db: &'a Vec<Value>, shortcut: &str) -> Option<&'a Value> {
    for entry in scribe_db {
        if let Some(entry_shortcut) = entry.get("shortcut").and_then(|s| s.as_str()) {
            if entry_shortcut == shortcut {
                return entry.get("snippet");
            }
        }
    }
    None
}

pub fn stop_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if !pid_file.exists() {
        println!("scribe daemon is not running");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    if let Ok(pid) = pid_str.trim().parse::<u32>() {
        // Send termination signal
        #[cfg(unix)]
        {
            let status = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("Stopped scribe daemon with PID {}", pid);
                // Remove PID file
                fs::remove_file(&pid_file)?;
            } else {
                println!("Failed to stop scribe daemon with PID {}", pid);
            }
        }

        // For Windows
        #[cfg(windows)]
        {
            use std::process::Command;
            let status = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("Stopped scribe daemon with PID {}", pid);
                // Remove PID file
                fs::remove_file(&pid_file)?;
            } else {
                println!("Failed to stop scribe daemon with PID {}", pid);
            }
        }
    } else {
        println!("Invalid PID in daemon file");
    }

    Ok(())
}

pub fn daemon_status() -> Result<(), Box<dyn std::error::Error>> {
    let pid_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let pid_file = pid_dir.join("scribe-daemon.pid");

    if !pid_file.exists() {
        println!("scribe daemon is not running");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    if let Ok(pid) = pid_str.trim().parse::<u32>() {
        // Check if process is running
        #[cfg(unix)]
        {
            let status = std::process::Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .status();

            if status.is_ok() && status.unwrap().success() {
                println!("scribe daemon is running with PID {}", pid);
            } else {
                println!("scribe daemon is not running (stale PID file)");
                // Remove stale PID file
                fs::remove_file(&pid_file)?;
            }
        }

        // For Windows or fallback
        #[cfg(not(unix))]
        {
            println!("scribe daemon appears to be running with PID {}", pid);
        }
    } else {
        println!("Invalid PID in daemon file");
    }

    Ok(())
}
