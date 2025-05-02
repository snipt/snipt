use enigo::{Direction, Key, Keyboard};
use std::env;
use std::fs::{self, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
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
pub fn execute_snippet(
    to_delete: usize,
    content: &str,
    params: Option<&Vec<String>>,
) -> Result<()> {
    // Delete the trigger and shortcut
    let mut keyboard = create_keyboard_controller()?;
    send_backspace(&mut keyboard, to_delete)?;

    if content.trim().is_empty() {
        return Err(SniptError::Other(
            "Cannot execute empty content".to_string(),
        ));
    }

    // Small delay to ensure UI state is stable
    thread::sleep(Duration::from_millis(10));

    // Execute based on content type
    if is_url(content) {
        // For URLs, we can safely spawn a thread since we don't need Enigo
        let url = content.to_string();

        // Spawn a separate thread only for URL opening to keep UI responsive
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open")
                .arg(&url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open")
                .arg(&url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(&["/c", "start", "", &url])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return Ok(());
        }
    } else if is_script(content) {
        // Execute script and type its output
        return execute_script(&mut keyboard, content, params);
    } else if content.contains('\n') || content.contains(';') {
        // Execute command and type its output
        return execute_command(&mut keyboard, content, params);
    } else {
        // For simple strings, check if we have parameters
        if let Some(params) = params {
            // Try to format the content with parameters
            let mut formatted_content = content.to_string();

            // Simple parameter substitution - replace $1, $2, etc. with parameter values
            for (i, param) in params.iter().enumerate() {
                let param_marker = format!("${}", i + 1);
                formatted_content = formatted_content.replace(&param_marker, param);
            }

            // Also handle expressions like ${1+2} by executing them
            if formatted_content.contains("${") && formatted_content.contains("}") {
                let modified_content = format!("#!/bin/bash\necho \"{}\"", formatted_content);
                return execute_script(&mut keyboard, &modified_content, None);
            }

            return type_text_with_formatting(&mut keyboard, &formatted_content);
        } else {
            // Just do normal text expansion for simple strings
            return type_text_with_formatting(&mut keyboard, content);
        }
    }
}

/// Execute content as a command directly in the current shell
fn execute_command(
    keyboard: &mut impl Keyboard,
    command: &str,
    params: Option<&Vec<String>>,
) -> Result<()> {
    // Apply parameter substitution if params are provided
    let command = if let Some(params) = params {
        apply_parameter_substitution(command, params)
    } else {
        command.to_string()
    };

    // Create a command with proper pipes to avoid shell window flashing
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");

    #[cfg(target_os = "windows")]
    cmd.args(["/c", &command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command_obj = Command::new(&shell);
        command_obj
            .args(["-c", &command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        command_obj
    };

    // Execute with timeout protection
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => return Err(SniptError::Io(e)),
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Trim trailing newlines to prevent execution
        let trimmed_stdout = stdout.trim_end().to_string();

        // Wait a tiny bit to ensure we're ready to type
        thread::sleep(Duration::from_millis(10));

        // Type the output
        type_text_with_formatting(keyboard, &trimmed_stdout)
    } else {
        Err(SniptError::Other(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

fn execute_script(
    keyboard: &mut impl Keyboard,
    script_content: &str,
    params: Option<&Vec<String>>,
) -> Result<()> {
    // Apply parameter substitution if params are provided
    let script_content = if let Some(params) = params {
        apply_parameter_substitution(script_content, params)
    } else {
        script_content.to_string()
    };

    // Prepare temp file
    let mut file = NamedTempFile::new()?;
    file.write_all(script_content.as_bytes())?;
    file.flush()?;

    let path = file.path().to_path_buf();

    // Make the script executable on Unix platforms
    #[cfg(not(target_os = "windows"))]
    {
        fs::set_permissions(&path, Permissions::from_mode(0o755))?;
    }

    // Execute with proper stdio redirection
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");

    #[cfg(target_os = "windows")]
    cmd.args(["/c", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    #[cfg(not(target_os = "windows"))]
    let mut cmd = Command::new(&path);

    #[cfg(not(target_os = "windows"))]
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    // Execute and handle output
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => return Err(SniptError::Io(e)),
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Trim trailing newlines to prevent execution
        let trimmed_stdout = stdout.trim_end().to_string();

        // Detect multi-line output
        if trimmed_stdout.contains('\n') {
            // For multi-line output on macOS/Linux, we need to handle it differently
            // to prevent each line from being executed as a command
            #[cfg(not(target_os = "windows"))]
            {
                // First, write the content to a file in a location that will definitely exist
                let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                let output_path = format!("{}/snipt_output.txt", home_dir);

                // Write the output to a fixed location
                fs::write(&output_path, &trimmed_stdout)?;

                // Make sure the file is readable
                #[cfg(unix)]
                fs::set_permissions(&output_path, Permissions::from_mode(0o644))?;

                // Prepare a cat command that will display the file contents
                let cat_cmd = format!("cat \"{}\"", output_path);

                // Small delay to ensure UI stability
                thread::sleep(Duration::from_millis(10));

                // Type the cat command
                match keyboard.text(&cat_cmd) {
                    Ok(_) => {}
                    Err(err) => {
                        return Err(SniptError::Enigo(format!("Failed to type text: {}", err)))
                    }
                }

                // Press Enter to execute the cat command
                match keyboard.key(Key::Return, Direction::Click) {
                    Ok(_) => {}
                    Err(err) => {
                        return Err(SniptError::Enigo(format!("Failed to type key: {}", err)))
                    }
                }

                // Schedule deletion for a few seconds later to ensure the cat command has time to run
                let path_to_delete = output_path.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(2));
                    let _ = fs::remove_file(path_to_delete); // Ignore errors
                });

                return Ok(());
            }
        }

        // Small delay to ensure UI stability
        thread::sleep(Duration::from_millis(10));

        // Type the output
        type_text_with_formatting(keyboard, &trimmed_stdout)
    } else {
        Err(SniptError::Other(format!(
            "Script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

/// Apply parameter substitution to the script/command content
fn apply_parameter_substitution(content: &str, params: &[String]) -> String {
    let mut result = content.to_string();

    // Replace $1, $2, etc. with parameter values
    for (i, param) in params.iter().enumerate() {
        let param_marker = format!("${}", i + 1);
        result = result.replace(&param_marker, param);
    }

    // Also replace $* with all parameters joined by space
    result = result.replace("$*", &params.join(" "));

    result
}
