/// Verify if a process with the given PID is running
#[cfg(unix)]
pub fn verify_process_running(pid: u32) -> bool {
    use std::process::Command;

    // On Unix, we can use kill -0 to check if process exists
    let output = Command::new("kill")
        .args(&["-0", &pid.to_string()])
        .output();

    if let Ok(output) = output {
        output.status.success()
    } else {
        false
    }
}

#[cfg(windows)]
pub fn verify_process_running(pid: u32) -> bool {
    use std::process::Command;

    // On Windows, we can use tasklist to check if process exists
    let output = Command::new("tasklist")
        .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();

    if let Ok(output) = output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        output_str.contains(&pid.to_string())
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
pub fn detect_windows_terminal() -> String {
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

#[cfg(target_os = "linux")]
pub fn detect_linux_terminal() -> String {
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
pub fn detect_desktop_environment() -> String {
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

#[cfg(target_os = "macos")]
pub fn get_frontmost_terminal_app() -> String {
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
pub fn detect_terminal_app() -> String {
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
pub fn find_terminal_in_process_tree(pid: u32) -> String {
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
pub fn is_running_as_sudo() -> bool {
    std::env::var("SUDO_USER").is_ok()
}

#[cfg(target_os = "windows")]
pub fn is_defender_blocking_likely() -> bool {
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
