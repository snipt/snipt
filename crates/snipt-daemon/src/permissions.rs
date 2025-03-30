use snipt_core::{Result, SniptError};

#[cfg(target_os = "macos")]
use crate::process::detect_terminal_app;

#[cfg(target_os = "linux")]
use crate::process::{detect_desktop_environment, detect_linux_terminal, is_running_as_sudo};

#[cfg(target_os = "windows")]
use crate::process::{detect_windows_terminal, is_defender_blocking_likely};

pub fn check_and_request_permissions() -> Result<()> {
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
        let terminal = detect_windows_terminal();

        println!("⚠️  snipt needs to monitor keyboard input to detect expansion triggers");
        println!("-----------------------------------------------------------------");
        println!(
            "You may see security warnings about {} accessing keyboard input.",
            terminal
        );
        println!();
        println!("This is normal and required for snipt's functionality:");
        println!("1. snipt monitors when you type the trigger character ':'");
        println!("2. It only processes text after the trigger to expand snippets");
        println!("3. snipt never logs or transmits your keystrokes");
        println!();

        // Check Windows Defender settings
        if is_defender_blocking_likely() {
            println!("Windows Security (Defender) may block this functionality.");
            println!("If snipt isn't working, you may need to add an exception:");
            println!("1. Open Windows Security > App & browser control");
            println!("2. Click 'Exploit protection settings'");
            println!("3. Go to 'Program settings' tab");
            println!("4. Add {} as an exception", terminal);
        }

        println!("\nSome antivirus programs may also block keyboard monitoring.");
        println!("If snipt doesn't work, check your antivirus settings.");

        println!("\nPress Enter to continue...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
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
const SWIFT_CHECK_PERMISSIONS: &str = r#"
import Foundation

// Simple function to check accessibility without AppKit dependencies
func checkAccessibility() -> Bool {
    let process = Process()
    process.launchPath = "/usr/bin/osascript"
    process.arguments = ["-e", "tell application \"System Events\" to return name of first process"]

    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = pipe

    do {
        try process.run()
        process.waitUntilExit()

        if process.terminationStatus == 0 {
            // If we got output, we have accessibility permissions
            return true
        } else {
            return false
        }
    } catch {
        return false
    }
}

// Run the check and print the result so Rust can read it
let result = checkAccessibility()
print(result)
"#;

#[cfg(target_os = "macos")]
fn has_accessibility_permission() -> bool {
    use std::process::Command;

    // First try the Swift approach
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

    // Direct AppleScript test as a fallback
    let apple_script_test = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to return name of first process")
        .output();

    match apple_script_test {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn request_macos_permissions() -> Result<()> {
    // Determine which terminal application is currently in use
    let terminal_app = detect_terminal_app();

    println!("⚠️  snipt needs accessibility permissions to detect keyboard input");
    println!("---------------------------------------------------------------");
    println!("Please follow these steps:");
    println!("1. You'll be prompted to open System Settings/Preferences");
    println!("2. Go to Privacy & Security > Privacy > Accessibility");
    println!("   (On older macOS: Security & Privacy > Privacy > Accessibility)");
    println!("3. Click the lock icon at the bottom left to make changes");
    println!("4. Find and check the box next to '{}'", terminal_app);
    println!("5. If it's already checked, uncheck and recheck it");
    println!();
    println!("Note: On macOS 14 (Sonoma) or newer, you also need to:");
    println!(
        "- Grant permission in Input Monitoring for '{}'",
        terminal_app
    );
    println!();

    // Ask user to open System Settings manually to avoid API issues
    println!("Would you like to open System Settings now? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" {
        use std::process::Command;

        // Try both ways to open settings (for compatibility with different macOS versions)
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status();
    }

    println!("\nPress Enter once you've granted permission...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Check permission
    if !has_accessibility_permission() {
        println!("\nPermission not detected. Please try again.");
        println!("Make sure to:");
        println!(
            "1. Check the box next to '{}' in the Accessibility list",
            terminal_app
        );
        println!("2. Also grant permission in Input Monitoring (on macOS 14+)");

        println!("\nWould you like to try testing again? (y/n)");
        let mut retry = String::new();
        std::io::stdin().read_line(&mut retry)?;

        if retry.trim().to_lowercase() == "y" {
            if !has_accessibility_permission() {
                return Err(SniptError::PermissionDenied(format!(
                    "Accessibility permission not granted for {}. Please restart and try again.",
                    terminal_app
                )));
            }
        } else {
            return Err(SniptError::PermissionDenied(format!(
                "Setup aborted. Please restart snipt after granting permissions to {}.",
                terminal_app
            )));
        }
    }

    println!("\n✅ Permission granted successfully!");
    Ok(())
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
    // Try to detect terminal and display environment
    let terminal_info = detect_linux_terminal();
    let desktop_env = detect_desktop_environment();

    println!("⚠️  snipt needs permissions to read input devices");
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
    println!("2. Run snipt with sudo privileges (temporary solution):");
    println!("   sudo snipt start");

    if !is_running_as_sudo() {
        println!("\nWould you like to continue running with sudo? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            println!("Please restart snipt with sudo: sudo snipt start");
        } else {
            println!("Please add your user to the input group and log out/in.");
        }

        return Err(SniptError::PermissionDenied(
            "snipt needs input device permissions to function".to_string(),
        ));
    }

    Ok(())
}
