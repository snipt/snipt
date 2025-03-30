use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use snipt_core::{is_daemon_running, Result};
use snipt_ui::display_snipt_dashboard;
use std::io::stdout;

pub fn display_main_ui() -> Result<()> {
    // First check if daemon is running
    let daemon_status = is_daemon_running()?;

    // Fully reset terminal state before displaying dashboard
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
    println!("\x1B[2J\x1B[1;1H"); // Clear screen

    // Now launch the UI with the daemon status information
    display_snipt_dashboard(daemon_status)
}
