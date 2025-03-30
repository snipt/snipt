use crate::keyboard_listener::start_keyboard_listener;
use crate::permissions::check_and_request_permissions;
use crate::process::verify_process_running;
use snipt_core::config::{db_file_exists, ensure_config_dir, get_db_file_path, get_pid_file_path};
use snipt_core::{get_config_dir, is_daemon_running, load_snippets, Result, SniptError};
use snipt_server::server::http_server::stop_api_server;
use snipt_server::server::utils::{get_api_server_port, port_is_available, save_api_port};
use std::fs::{self, File};
use std::io::Write;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Start the daemon process
pub fn start_daemon(api_port: u16) -> Result<()> {
    // Check if daemon is already running
    if let Some(pid) = is_daemon_running()? {
        if verify_process_running(pid) {
            println!("Daemon is already running with PID {}.", pid);
        } else {
            // PID file exists but process is not running - clean up and restart
            println!("Found stale PID file. Cleaning up and starting new daemon...");
            let _ = fs::remove_file(get_pid_file_path());
            // Continue with starting a new daemon
        }
    }

    if is_daemon_running()?.is_none() {
        println!("Starting snipt daemon...");

        // Ensure config directory exists
        ensure_config_dir()?;

        // Check if database exists
        if !db_file_exists() {
            return Err(SniptError::DatabaseNotFound(
                get_db_file_path().to_string_lossy().to_string(),
            ));
        }

        // Check and request permissions if needed
        check_and_request_permissions()?;

        #[cfg(unix)]
        {
            use std::process::Command;

            // Create a new detached process for the daemon
            // Get the path to the current executable
            let current_exe = std::env::current_exe()?;

            // Start the daemon process detached with nohup
            let daemon_log_file = format!("{}/daemon_log.txt", get_config_dir().to_string_lossy());

            let cmd = format!(
                "nohup {} daemon-worker > {} 2>&1 &",
                current_exe.to_string_lossy(),
                daemon_log_file
            );

            Command::new("sh").arg("-c").arg(&cmd).status()?;

            // Wait for the daemon to start and create its PID file
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(100));
                if is_daemon_running()?.is_some() {
                    break;
                }
            }

            // Verify the daemon is running
            if let Some(pid) = is_daemon_running()? {
                if verify_process_running(pid) {
                    println!("Daemon started successfully with PID {}.", pid);
                } else {
                    return Err(SniptError::Other(format!(
                        "Daemon process failed to start. Check logs at {}",
                        daemon_log_file
                    )));
                }
            } else {
                return Err(SniptError::Other(format!(
                    "Daemon failed to start. Check logs at {}",
                    daemon_log_file
                )));
            }
        }

        #[cfg(windows)]
        {
            use std::process::Command;

            // Get the path to the current executable
            let current_exe = std::env::current_exe()?;

            // Start the daemon process detached
            let daemon_log_file = format!("{}\\daemon_log.txt", get_config_dir().to_string_lossy());

            let cmd = format!(
                "START /B \"snipt Daemon\" \"{}\" daemon-worker > \"{}\" 2>&1",
                current_exe.to_string_lossy(),
                daemon_log_file
            );

            Command::new("cmd").arg("/C").arg(&cmd).status()?;

            // Wait for the daemon to start and create its PID file
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(100));
                if is_daemon_running()?.is_some() {
                    break;
                }
            }

            // Verify the daemon is running
            if let Some(pid) = is_daemon_running()? {
                if verify_process_running(pid) {
                    println!("Daemon started successfully with PID {}.", pid);
                } else {
                    return Err(SniptError::Other(format!(
                        "Daemon process failed to start. Check logs at {}",
                        daemon_log_file
                    )));
                }
            } else {
                return Err(SniptError::Other(format!(
                    "Daemon failed to start. Check logs at {}",
                    daemon_log_file
                )));
            }
        }
    }

    // Now start the API server
    println!("Starting API server...");
    let mut current_port = api_port;

    // Find an available port
    for _ in 0..10 {
        if port_is_available(current_port) {
            break;
        }
        println!(
            "Port {} is busy, trying {}...",
            current_port,
            current_port + 1
        );
        current_port += 1;
    }

    // Save the API port info
    if let Err(e) = save_api_port(current_port) {
        println!("Warning: Failed to save API port information: {}", e);
    }

    // Start the API server in a separate process
    let current_exe = std::env::current_exe()?;

    #[cfg(unix)]
    {
        use std::process::Command;
        let log_file = format!("{}/api_server_log.txt", get_config_dir().to_string_lossy());

        let cmd = format!(
            "nohup {} serve --port {} > {} 2>&1 &",
            current_exe.to_string_lossy(),
            current_port,
            log_file
        );

        Command::new("sh").arg("-c").arg(&cmd).status()?;

        // Verify the server started by checking if the port is no longer available
        thread::sleep(Duration::from_secs(2));
        if !port_is_available(current_port) {
            println!("API server started on port {}.", current_port);
            println!(
                "You can access the server at: http://localhost:{}",
                current_port
            );
            return Ok(());
        } else {
            return Err(SniptError::Other(format!(
                "API server failed to start. Check log at {}",
                log_file
            )));
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let log_file = format!("{}\\api_server_log.txt", get_config_dir().to_string_lossy());

        let cmd = format!(
            "START /B \"snipt API Server\" \"{}\" serve --port {} > \"{}\" 2>&1",
            current_exe.to_string_lossy(),
            current_port,
            log_file
        );

        Command::new("cmd").arg("/C").arg(&cmd).status()?;

        // Verify the server started
        thread::sleep(Duration::from_secs(2));
        if !port_is_available(current_port) {
            println!("API server started on port {}.", current_port);
            println!(
                "You can access the server at: http://localhost:{}",
                current_port
            );
            return Ok(());
        } else {
            return Err(SniptError::Other(format!(
                "API server failed to start. Check log at {}",
                log_file
            )));
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        return Err(SniptError::Other(
            "Starting API server not supported on this platform".to_string(),
        ));
    }
}

/// Stop the daemon if it's running
pub fn stop_daemon() -> Result<()> {
    let pid_file = get_pid_file_path();

    if !pid_file.exists() {
        return Err(SniptError::DaemonNotRunning);
    }

    // Read the PID file
    let pid_str = match fs::read_to_string(&pid_file) {
        Ok(content) => content,
        Err(e) => {
            // If we can't read the PID file, attempt to remove it
            let _ = fs::remove_file(&pid_file);
            return Err(SniptError::Other(format!("Failed to read PID file: {}", e)));
        }
    };

    // Parse the PID
    let pid = match pid_str.trim().parse::<u32>() {
        Ok(pid) => pid,
        Err(_) => {
            // If PID is invalid, remove the file and return error
            let _ = fs::remove_file(&pid_file);
            return Err(SniptError::InvalidPid);
        }
    };

    println!("Attempting to stop daemon with PID {}...", pid);

    // First try to stop the API server
    let _ = stop_api_server();

    // Check if the process is actually running before attempting to kill it
    if !verify_process_running(pid) {
        println!("Process with PID {} is not running.", pid);
        // Clean up PID file anyway
        let _ = fs::remove_file(&pid_file);
        return Ok(());
    }

    #[cfg(unix)]
    {
        // On Unix, first try sending SIGTERM for graceful shutdown
        let mut success = false;

        // Try SIGTERM first
        if let Ok(status) = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status()
        {
            if status.success() {
                println!("Sent termination signal to daemon with PID {}", pid);
                success = true;
            }
        }

        // If SIGTERM didn't work, try SIGKILL after a short delay
        if !success || verify_process_running(pid) {
            thread::sleep(Duration::from_millis(500));

            if verify_process_running(pid) {
                println!("Daemon didn't terminate gracefully, using force kill...");

                if let Ok(status) = std::process::Command::new("kill")
                    .args(&["-9", &pid.to_string()])
                    .status()
                {
                    if status.success() {
                        println!("Force killed daemon with PID {}", pid);
                        success = true;
                    }
                }
            } else {
                success = true; // Process terminated after SIGTERM
            }
        }

        // Try to kill any potential child processes too
        let _ = std::process::Command::new("pkill")
            .args(&["-P", &pid.to_string()])
            .status();

        if success {
            let _ = fs::remove_file(&pid_file);
            println!("Daemon stopped successfully.");
            return Ok(());
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        let mut success = false;

        // On Windows, first try normal termination
        if let Ok(status) = Command::new("taskkill")
            .args(&["/PID", &pid.to_string()])
            .status()
        {
            if status.success() {
                println!("Sent termination signal to daemon with PID {}", pid);
                success = true;
            }
        }

        // If that didn't work, try forced termination
        if !success || verify_process_running(pid) {
            thread::sleep(Duration::from_millis(500));

            if verify_process_running(pid) {
                println!("Daemon didn't terminate gracefully, using force kill...");

                // Kill forcefully and also kill child processes (/T)
                if let Ok(status) = Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &pid.to_string()])
                    .status()
                {
                    if status.success() {
                        println!("Force killed daemon with PID {}", pid);
                        success = true;
                    }
                }
            } else {
                success = true; // Process terminated after normal taskkill
            }
        }

        if success {
            let _ = fs::remove_file(&pid_file);
            println!("Daemon stopped successfully.");
            return Ok(());
        }
    }

    // If we get here, all attempts failed
    println!("WARNING: Failed to stop daemon process. PID file will be removed anyway.");
    let _ = fs::remove_file(&pid_file);

    Ok(())
}

/// Check daemon status
pub fn daemon_status() -> Result<()> {
    match is_daemon_running()? {
        Some(pid) => {
            // Verify the process is actually running
            let process_exists = verify_process_running(pid);

            if process_exists {
                println!("snipt daemon is running with PID {}", pid);

                // Check if we can find the API port information
                if let Ok(port) = get_api_server_port() {
                    println!("API server is running on port {}", port);
                    println!("UI available at: http://localhost:{}", port);
                }

                Ok(())
            } else {
                // Process not running but PID file exists
                println!("PID file exists but process {} is not running", pid);
                println!("This could indicate the daemon crashed or was stopped abruptly");
                println!("Recommend running 'snipt stop' followed by 'snipt start'");
                Ok(())
            }
        }
        None => {
            println!("snipt daemon is not running");
            Ok(())
        }
    }
}

/// The daemon worker process (run by the daemon itself)
#[cfg(unix)]
pub fn daemon_worker() -> Result<()> {
    // Create PID file
    let pid_file = get_pid_file_path();
    let mut file = File::create(&pid_file)?;
    write!(file, "{}", process::id())?;

    // Run the actual daemon worker
    run_daemon_worker()
}

/// The actual daemon worker process
pub fn run_daemon_worker() -> Result<()> {
    // Load snippets
    let db_path = get_db_file_path();
    if !db_path.exists() {
        return Err(SniptError::DatabaseNotFound(
            db_path.to_string_lossy().to_string(),
        ));
    }

    // Load the snipt database
    let snippets = Arc::new(Mutex::new(load_snippets()?));

    // Track the last modified time of the database file
    let last_modified = Arc::new(Mutex::new(fs::metadata(&db_path)?.modified().ok()));

    // Track running state
    let running = Arc::new(Mutex::new(true));
    let running_clone = Arc::clone(&running);

    // Start keyboard event listener in a separate thread
    let keyboard_thread = start_keyboard_listener(Arc::clone(&snippets), running_clone);

    // Clone references for the monitoring thread
    let db_path_clone = db_path.clone();
    let snippets_clone = Arc::clone(&snippets);
    let last_modified_clone = Arc::clone(&last_modified);

    // Monitor for database changes and termination signals
    let check_interval = Duration::from_secs(1);
    while *running.lock().unwrap() {
        // Add a small sleep to reduce CPU usage
        thread::sleep(Duration::from_millis(100));

        // Check if it's time to check for file changes
        static mut LAST_CHECK: Option<std::time::Instant> = None;
        let should_check = unsafe {
            let now = std::time::Instant::now();
            let check = match LAST_CHECK {
                Some(last) => now.duration_since(last) >= check_interval,
                None => true,
            };
            if check {
                LAST_CHECK = Some(now);
            }
            check
        };

        if should_check {
            // Check if the database file has been modified
            if let Ok(metadata) = fs::metadata(&db_path_clone) {
                if let Ok(current_modified) = metadata.modified() {
                    let reload_needed = {
                        let mut last_mod = last_modified_clone.lock().unwrap();
                        if let Some(last_mod_time) = *last_mod {
                            if current_modified > last_mod_time {
                                *last_mod = Some(current_modified);
                                true
                            } else {
                                false
                            }
                        } else {
                            *last_mod = Some(current_modified);
                            false
                        }
                    };

                    if reload_needed {
                        // Reload snippets
                        if let Ok(new_snippets) = load_snippets() {
                            let mut snippets_guard = snippets_clone.lock().unwrap();
                            *snippets_guard = new_snippets;
                        }
                    }
                }
            }
        }
    }

    // Wait for keyboard thread to finish
    if let Err(e) = keyboard_thread.join() {
        eprintln!("Error joining keyboard thread: {:?}", e);
    }

    Ok(())
}

/// This function runs as a separate daemon process
pub fn daemon_worker_entry() -> Result<()> {
    // Create PID file with the current process ID
    let pid_file = get_pid_file_path();
    let mut file = File::create(&pid_file)?;
    write!(file, "{}", process::id())?;

    // Run the actual daemon worker
    let result = run_daemon_worker();

    // Clean up PID file on exit
    let _ = fs::remove_file(&pid_file);

    result
}
