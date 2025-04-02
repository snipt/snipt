use snipt_core::{Result, SniptError};

#[cfg(target_os = "macos")]
use crate::process::{
    detect_terminal_app, find_terminal_in_process_tree, get_frontmost_terminal_app,
};

#[cfg(target_os = "linux")]
use crate::process::{
    detect_desktop_environment, detect_linux_terminal, is_running_as_sudo, verify_process_running,
};

#[cfg(target_os = "windows")]
use crate::process::{
    detect_windows_terminal, is_defender_blocking_likely, verify_process_running,
};

pub fn check_and_request_permissions() -> Result<()> {
    // Check permissions immediately at startup
    #[cfg(target_os = "macos")]
    {
        // Direct check for accessibility permissions for the snipt binary
        if !has_accessibility_permission() {
            return request_macos_permissions();
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Check input device permissions
        if !has_input_permission() {
            return request_linux_permissions();
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows doesn't have explicit permissions but we can inform about security software
        return inform_windows_permissions();
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn has_accessibility_permission() -> bool {
    use std::process::Command;

    // Simple test: Try to use basic UI scripting via AppleScript
    let result = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to key code 0")
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn request_macos_permissions() -> Result<()> {
    use std::process::Command;

    // Find the terminal that needs permissions
    let terminal_info = detect_terminal_info();
    let terminal_name = &terminal_info.name;
    let terminal_path = &terminal_info.path;

    // Print very clear instructions with the specific terminal details
    println!("\n⚠️ ACCESSIBILITY PERMISSION REQUIRED ⚠️");
    println!("=====================================");
    println!("snipt needs accessibility permissions to detect keyboard input.");
    println!();
    println!("Detected terminal: {}", terminal_name);
    if !terminal_path.is_empty() {
        println!("Terminal location: {}", terminal_path);
    }
    println!();

    println!("Since snipt runs inside your terminal, you need to grant");
    println!("ACCESSIBILITY PERMISSION to your TERMINAL APPLICATION.");
    println!();

    println!("Step-by-step instructions:");
    println!("------------------------");
    println!("1. We'll open System Settings > Privacy & Security > Accessibility");
    println!("2. Click the lock icon in the bottom left (if it's locked)");
    println!("3. Click the '+' button to add an application");
    println!("4. Navigate to your terminal:");

    if terminal_name.contains("iTerm") {
        println!("   - Press Shift+Command+A to go to Applications");
        println!("   - Find and select iTerm.app");
    } else if terminal_name.contains("Terminal") {
        println!("   - Press Shift+Command+A to go to Applications");
        println!("   - Open the Utilities folder");
        println!("   - Find and select Terminal.app");
    } else {
        println!("   - Navigate to where your terminal app is installed");
        println!("   - Select {}", terminal_name);
    }

    println!("5. Click 'Open' to add it to the list");
    println!(
        "6. Make sure the checkbox next to {} is CHECKED",
        terminal_name
    );
    println!();
    println!("IMPORTANT: After granting permission, you MUST:");
    println!("- Completely quit {} (Command+Q)", terminal_name);
    println!("- Restart {} completely", terminal_name);
    println!("- Run snipt again");
    println!();

    println!("Press Enter to open System Settings...");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Open System Settings to the Accessibility section
    let open_result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status();

    if open_result.is_err() {
        // Fallback for older macOS versions
        let _ = Command::new("open")
            .args(&["/System/Library/PreferencePanes/Security.prefPane"])
            .status();
    }

    // Show additional guidance for finding the app
    println!("\nNavigating to your terminal application:");

    if !terminal_path.is_empty() {
        println!("Copy this path to find your terminal: {}", terminal_path);

        // Create a "copy to clipboard" instruction based on the detected terminal
        if terminal_name.contains("iTerm") {
            println!("In iTerm, you can copy this path by selecting it and pressing Command+C");
        } else {
            println!("Select the path and copy it with Command+C");
        }

        println!("\nIn the file picker dialog:");
        println!("1. Press Command+Shift+G to 'Go to Folder'");
        println!("2. Paste the path and click 'Go'");
    }

    println!("\nAfter adding your terminal and checking the box:");
    println!("1. Quit {} completely with Command+Q", terminal_name);
    println!("2. Restart {} and run snipt again", terminal_name);
    println!();
    println!("The permissions will NOT take effect until you restart your terminal!");

    if !verify_permissions() {
        Err(SniptError::PermissionDenied(format!(
            "Accessibility permission required for {}. Please restart your terminal after granting permissions.",
            terminal_name
        )))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
struct TerminalInfo {
    name: String,
    path: String,
}

#[cfg(target_os = "macos")]
fn detect_terminal_info() -> TerminalInfo {
    use std::process::Command;

    // Try to get the terminal name using our existing functions
    let terminal_name = get_terminal_app_name();
    let mut terminal_path = String::new();

    // Look up the path based on the terminal name
    if terminal_name.contains("iTerm") {
        // Check common iTerm locations
        let possible_paths = [
            "/Applications/iTerm.app",
            "/Applications/iTerm2.app",
            "~/Applications/iTerm.app",
            "~/Applications/iTerm2.app",
        ];

        for path in possible_paths.iter() {
            let expanded_path = if path.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    format!("{}{}", home, &path[1..])
                } else {
                    path.to_string()
                }
            } else {
                path.to_string()
            };

            if std::path::Path::new(&expanded_path).exists() {
                terminal_path = expanded_path;
                break;
            }
        }

        // If not found in common locations, try to find it
        if terminal_path.is_empty() {
            if let Ok(output) = Command::new("mdfind")
                .args(&["-name", "iTerm.app"])
                .output()
            {
                let result = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = result.lines().next() {
                    if !first_line.is_empty() {
                        terminal_path = first_line.to_string();
                    }
                }
            }

            // Try iTerm2.app if iTerm.app not found
            if terminal_path.is_empty() {
                if let Ok(output) = Command::new("mdfind")
                    .args(&["-name", "iTerm2.app"])
                    .output()
                {
                    let result = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = result.lines().next() {
                        if !first_line.is_empty() {
                            terminal_path = first_line.to_string();
                        }
                    }
                }
            }
        }
    } else if terminal_name.contains("Terminal") {
        // Standard path for Terminal.app
        let standard_path = "/System/Applications/Utilities/Terminal.app";
        let old_path = "/Applications/Utilities/Terminal.app";

        if std::path::Path::new(standard_path).exists() {
            terminal_path = standard_path.to_string();
        } else if std::path::Path::new(old_path).exists() {
            terminal_path = old_path.to_string();
        }
    } else {
        // For other terminals, try to find them using mdfind
        let search_name = if terminal_name.ends_with(".app") {
            terminal_name.clone()
        } else {
            format!("{}.app", terminal_name)
        };

        if let Ok(output) = Command::new("mdfind")
            .args(&["-name", &search_name])
            .output()
        {
            let result = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = result.lines().next() {
                if !first_line.is_empty() {
                    terminal_path = first_line.to_string();
                }
            }
        }
    }

    TerminalInfo {
        name: terminal_name,
        path: terminal_path,
    }
}

#[cfg(target_os = "macos")]
fn get_terminal_app_name() -> String {
    // Try multiple methods to identify the terminal
    let frontmost = get_frontmost_terminal_app();
    if !frontmost.is_empty() {
        return clean_terminal_name(&frontmost);
    }

    let terminal = detect_terminal_app();
    if !terminal.contains("your terminal application") {
        return clean_terminal_name(&terminal);
    }

    // Get the parent process ID
    let ppid = match std::process::Command::new("ps")
        .args(&["-o", "ppid=", "-p", &format!("{}", std::process::id())])
        .output()
    {
        Ok(output) => {
            let ppid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            ppid_str.parse::<u32>().unwrap_or(0)
        }
        Err(_) => 0,
    };

    if ppid > 0 {
        let tree_terminal = find_terminal_in_process_tree(ppid);
        if !tree_terminal.is_empty() {
            return clean_terminal_name(&tree_terminal);
        }
    }

    // Try to get process information directly
    if let Ok(output) = std::process::Command::new("ps")
        .args(&["-p", &std::process::id().to_string(), "-o", "comm="])
        .output()
    {
        let proc_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if proc_name.contains("iterm") || proc_name.contains("iTerm") {
            return "iTerm.app".to_string();
        } else if proc_name.contains("Terminal") {
            return "Terminal.app".to_string();
        }
    }

    // Default to Terminal.app if detection fails
    "Terminal.app".to_string()
}

#[cfg(target_os = "macos")]
fn clean_terminal_name(name: &str) -> String {
    // Ensure terminal name ends with .app for clarity
    if name.ends_with(".app") {
        name.to_string()
    } else {
        // Check common terminal names and format them
        if name == "Terminal" {
            "Terminal.app".to_string()
        } else if name == "iTerm" || name == "iTerm2" {
            "iTerm.app".to_string()
        } else if name == "Alacritty" {
            "Alacritty.app".to_string()
        } else if name == "kitty" {
            "kitty.app".to_string()
        } else {
            format!("{}.app", name)
        }
    }
}

#[cfg(target_os = "linux")]
fn has_input_permission() -> bool {
    // Check if we have permission to access input devices
    use std::path::Path;

    // Try to read from /dev/input/event0 as a test
    if Path::new("/dev/input/event0").exists() {
        match std::fs::File::open("/dev/input/event0") {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        // If the file doesn't exist, check group membership
        let output = std::process::Command::new("groups")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(groups) = output {
            groups.contains("input")
        } else {
            false
        }
    }
}

#[cfg(target_os = "linux")]
fn request_linux_permissions() -> Result<()> {
    // Get the path to the snipt binary
    let snipt_binary_path = std::env::current_exe()
        .map_err(|e| SniptError::Other(format!("Failed to get executable path: {}", e)))?;

    let binary_name = snipt_binary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snipt");

    // Try to detect display environment and terminal using functions from process.rs
    let desktop_env = detect_desktop_environment();
    let terminal = detect_linux_terminal();

    println!("⚠️  snipt needs permissions to read input devices");
    println!("-----------------------------------------------");
    println!("Detected binary: {}", binary_name);
    println!("Desktop environment: {}", desktop_env);
    println!("Terminal: {}", terminal);
    println!();

    // Tailored instructions based on display environment
    match desktop_env.as_str() {
        "GNOME" => {
            println!("For GNOME users:");
            println!("1. You may need to allow input monitoring through GNOME settings");
            println!("2. Open Settings > Privacy > Input Monitoring");
            println!("3. Toggle on the snipt application");
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
    println!("2. Run snipt with sudo privileges (temporary solution):");
    println!("   sudo snipt start");

    // Using is_running_as_sudo() from process.rs
    if !is_running_as_sudo() {
        println!("\nWould you like to try granting permissions now? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            // Ask if they want to add themselves to input group
            println!("Would you like to add yourself to the input group? (y/n)");
            println!("Note: This requires sudo access and logging out/in to take effect.");
            let mut group_input = String::new();
            std::io::stdin().read_line(&mut group_input)?;

            if group_input.trim().to_lowercase() == "y" {
                // Try to add user to input group
                let username =
                    std::env::var("USER").unwrap_or_else(|_| "your_username".to_string());
                let status = std::process::Command::new("sudo")
                    .args(&["usermod", "-a", "-G", "input", &username])
                    .status();

                match status {
                    Ok(exit_status) if exit_status.success() => {
                        println!("✅ Successfully added user to input group!");
                        println!("You need to log out and log back in for this to take effect.");
                        println!("After logging back in, run 'snipt start' again.");
                    }
                    _ => {
                        println!("❌ Failed to add user to input group.");
                        println!("You can try running the command manually:");
                        println!("  sudo usermod -a -G input {}", username);
                    }
                }
            } else {
                // Suggest running with sudo
                println!("You can try running snipt with sudo privileges:");
                println!("  sudo snipt start");
            }

            return Err(SniptError::PermissionDenied(
                "snipt needs input device permissions to function. Please restart after fixing permissions.".to_string(),
            ));
        } else {
            println!("Permission not granted. snipt may not function correctly.");
            return Err(SniptError::PermissionDenied(
                "snipt needs input device permissions to function".to_string(),
            ));
        }
    }

    // If we're already running as sudo, check if we have access now
    if has_input_permission() {
        println!("✅ Input device permissions verified!");
        return Ok(());
    } else {
        println!("❌ Still unable to access input devices, even with sudo.");
        println!("This might be due to additional security measures on your system.");
        return Err(SniptError::PermissionDenied(
            "Unable to access input devices even with elevated privileges".to_string(),
        ));
    }
}

#[cfg(target_os = "windows")]
fn inform_windows_permissions() -> Result<()> {
    // Get the path to the snipt binary
    let snipt_binary_path = std::env::current_exe()
        .map_err(|e| SniptError::Other(format!("Failed to get executable path: {}", e)))?;

    let binary_name = snipt_binary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snipt.exe");

    // Using detect_windows_terminal() from process.rs
    let terminal = detect_windows_terminal();

    println!("⚠️  snipt needs to monitor keyboard input to detect expansion triggers");
    println!("-----------------------------------------------------------------");
    println!("Detected Terminal: {}", terminal);
    println!(
        "You may see security warnings about {} accessing keyboard input.",
        binary_name
    );
    println!();
    println!("This is normal and required for snipt's functionality:");
    println!("1. snipt monitors when you type the trigger character ':'");
    println!("2. It only processes text after the trigger to expand snippets");
    println!("3. snipt never logs or transmits your keystrokes");
    println!();

    // Check Windows Defender settings using is_defender_blocking_likely() from process.rs
    if is_defender_blocking_likely() {
        println!("ℹ️  Windows Security (Defender) may block this functionality.");
        println!("If snipt isn't working, you may need to add an exception:");
        println!("1. Open Windows Security > App & browser control");
        println!("2. Click 'Exploit protection settings'");
        println!("3. Go to 'Program settings' tab");
        println!("4. Add {} as an exception", binary_name);

        println!("\nWould you like to open Windows Security now? (y/n)");
        let mut security_input = String::new();
        std::io::stdin().read_line(&mut security_input)?;

        if security_input.trim().to_lowercase() == "y" {
            // Try to open Windows Security
            let _ = std::process::Command::new("cmd")
                .args(&["/C", "start", "windowsdefender:"])
                .status();
        }
    }

    println!("\nSome antivirus programs may also block keyboard monitoring.");
    println!(
        "If snipt doesn't work, check your antivirus settings for {}.",
        binary_name
    );

    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // On Windows, we can't easily verify permissions, so just proceed
    println!("✅ Setup complete. If you encounter issues with keyboard detection,");
    println!(
        "   you may need to configure your security software to allow {}.",
        binary_name
    );

    Ok(())
}

// Add a reusable permissions verification function
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn verify_permissions() -> bool {
    // For platforms where permissions could be revoked at runtime
    // Rerun our permission checks to see if they're still valid

    #[cfg(target_os = "linux")]
    {
        has_input_permission()
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, we don't have a reliable way to check if we still have permissions
        // So we'll assume we do, but we could add more checks here
        true
    }
}

#[cfg(target_os = "macos")]
pub fn verify_permissions() -> bool {
    // For macOS, check if we still have accessibility permissions
    has_accessibility_permission()
}
