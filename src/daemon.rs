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

#[cfg(target_os = "macos")]
const SWIFT_CHECK_PERMISSIONS: &str = r#"
import Foundation
import AppKit

@discardableResult
func checkAccessibilityPermissions() -> Bool {
    let checkOptPrompt = "AXTrustedCheckOptionPrompt"
    let options = [checkOptPrompt: false] as CFDictionary

    // Call AXIsProcessTrustedWithOptions through dlsym to avoid direct linkage
    let axlib = dlopen("/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices", RTLD_LAZY)
    defer { if axlib != nil { dlclose(axlib) } }

    typealias AXIsProcessTrustedWithOptionsType = @convention(c) (CFDictionary?) -> Bool
    let axFuncPtr = dlsym(axlib, "AXIsProcessTrustedWithOptions")

    var accessEnabled = false
    if axFuncPtr != nil {
        let axFunc = unsafeBitCast(axFuncPtr, to: AXIsProcessTrustedWithOptionsType.self)
        accessEnabled = axFunc(options)
    } else {
        // Fallback to UI check
        let workspace = NSWorkspace.shared
        let runningApps = workspace.runningApplications
        let secPrefsRunning = runningApps.contains(where: { $0.bundleIdentifier == "com.apple.systempreferences" })
        accessEnabled = secPrefsRunning
    }

    print(accessEnabled)
    return accessEnabled
}

checkAccessibilityPermissions()
"#;

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

    // Check and request permissions if needed
    check_and_request_permissions()?;

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

fn check_and_request_permissions() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Check macOS accessibility permissions
        if !has_accessibility_permission() {
            request_macos_permissions()?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows doesn't typically require explicit permissions for keyboard monitoring
        // but we can inform the user about potential antivirus alerts
        println!("Note: Your antivirus software may alert about keyboard monitoring.");
        println!("This is required for Scribe to detect text expansion triggers.");
    }

    #[cfg(target_os = "linux")]
    {
        // Check if running with appropriate permissions for input events
        if !has_input_permission() {
            request_linux_permissions()?;
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn has_accessibility_permission() -> bool {
    use std::process::Command;

    // First try the scripting approach
    let swift_result = Command::new("swift")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(SWIFT_CHECK_PERMISSIONS.as_bytes())?;
            }
            child.wait_with_output()
        });

    if let Ok(output) = swift_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() == "true" {
            return true;
        }
    }

    // Fallback approach: try to access input monitoring directly
    let test_result = Command::new("sh")
        .arg("-c")
        .arg(r#"osascript -e 'tell application "System Events" to keystroke ""' 2>/dev/null && echo success || echo failed"#)
        .output();

    if let Ok(output) = test_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout.trim() == "success";
    }

    false
}

#[cfg(target_os = "macos")]
fn request_macos_permissions() -> Result<()> {
    // Determine which terminal application is currently in use
    let terminal_app = detect_terminal_app();

    println!("⚠️  Scribe needs accessibility permissions to detect keyboard input");
    println!("---------------------------------------------------------------");
    println!("Please follow these steps:");
    println!("1. System Settings will open to: Privacy & Security > Privacy > Accessibility");
    println!("   (On older macOS versions: Security & Privacy > Privacy > Accessibility)");
    println!("2. Click the lock icon at the bottom left to make changes");
    println!("3. Look for your terminal app ('{}')", terminal_app);
    println!("4. Check the box next to your terminal application");
    println!("5. If it's already checked, uncheck and recheck it");
    println!();
    println!("Scribe runs inside your terminal, so it's your terminal app");
    println!("that needs permission, not your shell or Scribe itself.");
    println!();
    println!("Note: On macOS 14 (Sonoma) or newer, you may need to:");
    println!("- Go to Privacy & Security > Input Monitoring as well");
    println!("- Grant permissions there too for your terminal app");

    use std::process::Command;

    // Try both ways to open settings (for compatibility with different macOS versions)
    let _ = Command::new("open")
        .args(&["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"])
        .status();

    // For newer macOS versions (Sonoma+)
    let _ = Command::new("open")
        .args(&["x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility"])
        .status();

    println!("\nPress Enter once you've granted permission...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !has_accessibility_permission() {
        // One more attempt to open settings with more specific instructions
        println!("\nPermission not detected. Please try again.");
        println!("Make sure to:");
        println!(
            "1. Check the box next to '{}' in the Accessibility list",
            terminal_app
        );
        println!("2. Also grant permission in Input Monitoring (on macOS 14+)");
        println!("3. If '{}' isn't listed, click '+' to add it", terminal_app);

        // Ask again
        println!("\nPress Enter once you've completed these steps...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !has_accessibility_permission() {
            return Err(ScribeError::PermissionDenied(format!(
                "Accessibility permission not granted for {}. Please restart and try again.",
                terminal_app
            )));
        }
    }

    println!("\n✅ Permission granted successfully!");
    Ok(())
}

#[cfg(target_os = "macos")]
fn detect_terminal_app() -> String {
    use std::process::Command;

    // First, try to get the frontmost terminal application name directly
    let frontmost_app = get_frontmost_terminal_app();
    if !frontmost_app.is_empty() {
        return frontmost_app;
    }

    // Get the parent process ID to determine the terminal
    let ppid = match Command::new("ps")
        .args(&["-o", "ppid=", "-p", &format!("{}", std::process::id())])
        .output()
    {
        Ok(output) => {
            let ppid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            ppid_str.parse::<u32>().unwrap_or(0)
        }
        Err(_) => 0,
    };

    // Follow the process tree up to find the terminal application
    let terminal_app = find_terminal_in_process_tree(ppid);
    if !terminal_app.is_empty() {
        return terminal_app;
    }

    // If we still haven't found it, list all possible terminals for the user
    "your terminal application (Terminal.app, iTerm2, etc.)".to_string()
}

#[cfg(target_os = "macos")]
fn get_frontmost_terminal_app() -> String {
    // Try to get the frontmost app using AppleScript
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output();

    if let Ok(output) = output {
        let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Map known terminal app names to their proper name
        match app_name.as_str() {
            "iTerm" | "iTerm2" => return "iTerm2".to_string(),
            "Terminal" => return "Terminal".to_string(),
            "Alacritty" => return "Alacritty".to_string(),
            "kitty" => return "kitty".to_string(),
            "Hyper" => return "Hyper".to_string(),
            "WezTerm" => return "WezTerm".to_string(),
            _ => {
                if app_name.contains("Terminal") || app_name.contains("Console") {
                    return format!("{}.app", app_name);
                }
            }
        }
    }

    String::new() // Return empty string if we couldn't determine the app
}

#[cfg(target_os = "macos")]
fn find_terminal_in_process_tree(pid: u32) -> String {
    use std::process::Command;

    // No need to check if pid is zero
    if pid == 0 {
        return String::new();
    }

    // Get process command name
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "comm="])
        .output();

    if let Ok(output) = output {
        let comm = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Check if this is a known terminal app
        match comm.as_str() {
            "iTerm2" | "iterm2" => return "iTerm2".to_string(),
            "iTerm" => return "iTerm".to_string(),
            "Terminal" => return "Terminal".to_string(),
            "Alacritty" => return "Alacritty".to_string(),
            "kitty" => return "kitty".to_string(),
            "Hyper" => return "Hyper".to_string(),
            "WezTerm" => return "WezTerm".to_string(),
            _ => {}
        }

        // If it's a shell, get the parent and continue
        if comm.contains("sh")
            || comm == "bash"
            || comm == "zsh"
            || comm == "fish"
            || comm.starts_with('-')
        {
            // Get parent PID
            let ppid_output = Command::new("ps")
                .args(&["-p", &pid.to_string(), "-o", "ppid="])
                .output();

            if let Ok(ppid_output) = ppid_output {
                let ppid_str = String::from_utf8_lossy(&ppid_output.stdout)
                    .trim()
                    .to_string();
                if let Ok(ppid) = ppid_str.parse::<u32>() {
                    if ppid != 0 && ppid != pid {
                        return find_terminal_in_process_tree(ppid);
                    }
                }
            }
        }

        // Special case for recognizing terminal app names even if they don't match the standard names
        if comm.to_lowercase().contains("term") && !comm.contains("daemon") {
            return format!("{}.app", comm);
        }
    }

    // If we still can't determine, check if there are any obvious terminal apps running
    let terminal_check = Command::new("ps").args(&["-e", "-o", "comm="]).output();

    if let Ok(output) = terminal_check {
        let all_processes = String::from_utf8_lossy(&output.stdout);
        for line in all_processes.lines() {
            match line.trim() {
                "iTerm2" => return "iTerm2".to_string(),
                "iTerm" => return "iTerm".to_string(),
                "Terminal" => return "Terminal".to_string(),
                _ => {}
            }
        }
    }

    String::new()
}

#[cfg(target_os = "linux")]
fn request_linux_permissions() -> Result<()> {
    // Try to detect terminal and display environment
    let terminal_info = detect_linux_terminal();
    let desktop_env = detect_desktop_environment();

    println!("⚠️  Scribe needs permissions to read input devices");
    println!("-----------------------------------------------");
    println!("Detected terminal: {}", terminal_info);
    println!("Desktop environment: {}", desktop_env);
    println!();

    // Tailored instructions based on display environment
    match desktop_env.as_str() {
        "GNOME" => {
            println!("For GNOME users:");
            println!("1. You may need to allow input monitoring through GNOME settings");
            println!("2. Open Settings > Privacy > Input Monitoring");
            println!("3. Toggle on your terminal application ({})", terminal_info);
        }
        "KDE" => {
            println!("For KDE Plasma users:");
            println!("1. KDE generally doesn't restrict input monitoring by default");
            println!("2. If you have issues, check System Settings > Privacy");
        }
        _ => {
            // Generic Linux instructions
            println!("You can grant input access in one of these ways:");
        }
    }

    println!();
    println!("Alternatively, you can use one of these methods:");
    println!("1. Add your user to the 'input' group (recommended):");
    println!("   sudo usermod -a -G input $USER");
    println!("   (You'll need to log out and back in for this to take effect)");
    println!();
    println!("2. Run Scribe with sudo privileges (temporary solution):");
    println!("   sudo scribe start");

    if !is_running_as_sudo() {
        println!("\nWould you like to continue running with sudo? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            println!("Please restart Scribe with sudo: sudo scribe start");
        } else {
            println!("Please add your user to the input group and log out/in.");
        }

        return Err(ScribeError::PermissionDenied(
            "Scribe needs input device permissions to function".to_string(),
        ));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn detect_linux_terminal() -> String {
    use std::env;

    // Try to detect terminal from environment variables
    if let Ok(term) = env::var("TERM_PROGRAM") {
        return term;
    }

    // Try to get from process tree
    let output = std::process::Command::new("ps")
        .args(&["-p", &format!("{}", std::process::id()), "-o", "args="])
        .output();

    if let Ok(output) = output {
        let process_name = String::from_utf8_lossy(&output.stdout);
        if process_name.contains("gnome-terminal") {
            return "gnome-terminal".to_string();
        } else if process_name.contains("konsole") {
            return "konsole".to_string();
        } else if process_name.contains("xterm") {
            return "xterm".to_string();
        } else if process_name.contains("alacritty") {
            return "alacritty".to_string();
        } else if process_name.contains("kitty") {
            return "kitty".to_string();
        } else if process_name.contains("terminator") {
            return "terminator".to_string();
        }
    }

    // Fallback
    "your terminal emulator".to_string()
}

#[cfg(target_os = "linux")]
fn detect_desktop_environment() -> String {
    use std::env;

    // Common environment variables for desktop detection
    for var in &["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION", "GDMSESSION"] {
        if let Ok(val) = env::var(var) {
            if !val.is_empty() {
                if val.to_uppercase().contains("GNOME") {
                    return "GNOME".to_string();
                } else if val.to_uppercase().contains("KDE") {
                    return "KDE".to_string();
                } else if val.to_uppercase().contains("XFCE") {
                    return "XFCE".to_string();
                } else if val.to_uppercase().contains("CINNAMON") {
                    return "Cinnamon".to_string();
                } else if val.to_uppercase().contains("MATE") {
                    return "MATE".to_string();
                } else {
                    return val;
                }
            }
        }
    }

    // Fallback
    "Unknown".to_string()
}

#[cfg(target_os = "windows")]
fn check_and_request_permissions() -> Result<()> {
    // Detect Windows terminal
    let terminal = detect_windows_terminal();

    println!("⚠️  Scribe needs to monitor keyboard input to detect expansion triggers");
    println!("-----------------------------------------------------------------");
    println!(
        "You may see security warnings about {} accessing keyboard input.",
        terminal
    );
    println!();
    println!("This is normal and required for Scribe's functionality:");
    println!("1. Scribe monitors when you type the trigger character ':'");
    println!("2. It only processes text after the trigger to expand snippets");
    println!("3. Scribe never logs or transmits your keystrokes");
    println!();

    // Check Windows Defender settings
    if is_defender_blocking_likely() {
        println!("Windows Security (Defender) may block this functionality.");
        println!("If Scribe isn't working, you may need to add an exception:");
        println!("1. Open Windows Security > App & browser control");
        println!("2. Click 'Exploit protection settings'");
        println!("3. Go to 'Program settings' tab");
        println!("4. Add {} as an exception", terminal);
    }

    println!("\nSome antivirus programs may also block keyboard monitoring.");
    println!("If Scribe doesn't work, check your antivirus settings.");

    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn detect_windows_terminal() -> String {
    use std::env;

    // Try to get parent process name
    if let Ok(output) = std::process::Command::new("wmic.exe")
        .args(&[
            "process",
            "where",
            &format!("processid={}", std::process::id()),
            "get",
            "parentprocessid",
        ])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(ppid_str) = output_str.lines().nth(1) {
            let ppid = ppid_str.trim();
            if !ppid.is_empty() {
                if let Ok(parent_output) = std::process::Command::new("wmic.exe")
                    .args(&[
                        "process",
                        "where",
                        &format!("processid={}", ppid),
                        "get",
                        "name",
                    ])
                    .output()
                {
                    let parent_name = String::from_utf8_lossy(&parent_output.stdout);
                    if let Some(name) = parent_name.lines().nth(1) {
                        let name = name.trim();

                        // Map common Windows terminals
                        if name.eq_ignore_ascii_case("cmd.exe") {
                            return "Command Prompt (cmd.exe)".to_string();
                        } else if name.eq_ignore_ascii_case("powershell.exe")
                            || name.eq_ignore_ascii_case("pwsh.exe")
                        {
                            return "PowerShell".to_string();
                        } else if name.eq_ignore_ascii_case("windowsterminal.exe") {
                            return "Windows Terminal".to_string();
                        } else if name.eq_ignore_ascii_case("conhost.exe") {
                            return "Console Host".to_string();
                        } else if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }

    // Fallback
    "your terminal application".to_string()
}

#[cfg(target_os = "windows")]
fn is_defender_blocking_likely() -> bool {
    // Check if Exploit Guard might be enabled
    let output = std::process::Command::new("powershell.exe")
        .args(&[
            "-Command",
            "Get-MpPreference | Select-Object -ExpandProperty EnableExploitProtection",
        ])
        .output();

    if let Ok(output) = output {
        let result = String::from_utf8_lossy(&output.stdout).trim();
        return result == "True" || result == "1";
    }

    false
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

    // Track the last modified time of the database file
    let last_modified = Arc::new(Mutex::new(fs::metadata(&db_path)?.modified().ok()));

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
