use enigo::Keyboard;
use std::env;
use std::fs::{self, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::NamedTempFile;

use crate::expansion::type_text_with_formatting;
use crate::keyboard::{create_keyboard_controller, send_backspace};
use crate::{Result, SniptError};

/// Determine if a string is a URL
fn is_url(content: &str) -> bool {
    let content = content.trim();

    // Check for explicit protocols
    if content.starts_with("http://")
        || content.starts_with("https://")
        || content.starts_with("www.")
    {
        return true;
    }

    // Check for domain-like patterns (contains a dot followed by valid TLD characters)
    if content.contains('.') {
        let parts: Vec<&str> = content
            .split('/')
            .next()
            .unwrap()
            .split(':')
            .next()
            .unwrap()
            .split('.')
            .collect();
        if parts.len() >= 2 {
            // Check if the last part might be a TLD (2-63 chars, alphanumeric)
            let last_part = parts.last().unwrap();
            if last_part.len() >= 2
                && last_part.len() <= 63
                && last_part.chars().all(|c| c.is_alphanumeric() || c == '-')
            {
                return true;
            }
        }
    }

    false
}

/// Determine if the content should be treated as a script
fn is_script(content: &str) -> bool {
    // Check for shebang line
    content.trim().starts_with("#!")
}

/// Execute a snippet based on its content type
pub fn execute_snippet(to_delete: usize, content: &str) -> Result<()> {
    // Delete the trigger and shortcut
    let mut keyboard = create_keyboard_controller()?;
    send_backspace(&mut keyboard, to_delete + 1)?;

    if content.trim().is_empty() {
        return Err(SniptError::Other(
            "Cannot execute empty content".to_string(),
        ));
    }

    if is_url(content) {
        open_url(content)
    } else if is_script(content) {
        execute_script(&mut keyboard, content)
    } else {
        execute_command(&mut keyboard, content)
    }
}

/// Open a URL in the default browser
fn open_url(url: &str) -> Result<()> {
    // Ensure URL has proper scheme
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        // If it starts with www, prepend https://
        if url.starts_with("www.") {
            format!("https://{}", url)
        } else {
            // For other domain-like strings, add https://
            format!("https://{}", url)
        }
    } else {
        url.to_string()
    };

    println!("Opening URL: {}", url);

    // Use the appropriate open command based on the platform
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(&url).status();

    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(&url).status();

    #[cfg(target_os = "windows")]
    let status = Command::new("cmd").args(&["/c", "start", &url]).status();

    match status {
        Ok(exit_status) if exit_status.success() => Ok(()),
        Ok(exit_status) => Err(SniptError::Other(format!(
            "Failed to open URL: process exited with code {:?}",
            exit_status.code()
        ))),
        Err(e) => Err(SniptError::Io(e)),
    }
}

/// Execute content as a command directly in the current shell
fn execute_command(keyboard: &mut impl Keyboard, command: &str) -> Result<()> {
    println!("Executing command: {}", command);

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd").args(["/c", command]).output()?;

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        Command::new(&shell).args(["-c", command]).output()?
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Trim trailing newlines to prevent execution
        let trimmed_stdout = stdout.trim_end().to_string();
        type_text_with_formatting(keyboard, &trimmed_stdout)
    } else {
        Err(SniptError::Other(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

fn execute_script(keyboard: &mut impl Keyboard, script_content: &str) -> Result<()> {
    let mut file = NamedTempFile::new()?;
    file.write_all(script_content.as_bytes())?;
    file.flush()?;

    let path = file.path().to_path_buf();

    // Make the script executable on Unix platforms
    #[cfg(not(target_os = "windows"))]
    {
        fs::set_permissions(&path, Permissions::from_mode(0o755))?;
    }

    // Execute the script
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/c", path.to_str().unwrap()])
        .output()?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&path).output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Trim trailing newlines to prevent execution
        let trimmed_stdout = stdout.trim_end().to_string();
        type_text_with_formatting(keyboard, &trimmed_stdout)
    } else {
        Err(SniptError::Other(format!(
            "Script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}
