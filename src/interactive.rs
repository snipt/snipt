use crate::storage::add_snippet;
use crate::{error::Result, ScribeError};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

// Constants
const MAX_LINES: usize = 10000;
const MAX_LINE_LENGTH: usize = 5000;
const MAX_SHORTCUT_LENGTH: usize = 50;
const RENDER_THRESHOLD: Duration = Duration::from_millis(33); // Limit rendering frequency

#[derive(PartialEq, Copy, Clone)]
enum EditorMode {
    Normal,
    Insert,
    Paste, // New mode for pasting large text
}

pub enum AddResult {
    Added,
    Cancelled,
    Error(ScribeError),
}

pub fn interactive_add() -> AddResult {
    // Setup terminal with error handling
    if let Err(e) = terminal::enable_raw_mode() {
        return AddResult::Error(ScribeError::Other(format!(
            "Failed to enable raw mode: {}",
            e
        )));
    }

    let mut stdout = stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        terminal::disable_raw_mode().ok();
        return AddResult::Error(ScribeError::Other(format!(
            "Failed to enter alternate screen: {}",
            e
        )));
    }

    // Run the interactive UI
    let result = run_interactive_ui(&mut stdout);

    // Cleanup terminal
    let _ = execute!(stdout, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();

    match result {
        Ok(true) => AddResult::Added,
        Ok(false) => AddResult::Cancelled,
        Err(e) => AddResult::Error(e),
    }
}

fn run_interactive_ui(stdout: &mut io::Stdout) -> Result<bool> {
    let mut shortcut = String::new();
    let mut snippet = Vec::new();
    snippet.push(String::new());
    let mut current_field = 0; // 0 for shortcut, 1 for snippet
    let mut cursor_pos = 0;
    let mut current_line = 0;
    let mut editor_mode = EditorMode::Insert;
    let mut error_message = None;

    // For paste handling
    let mut paste_buffer = String::new();

    // For performance optimization
    let mut last_render = Instant::now();
    let mut force_render = true;
    let snippet_added = false;

    loop {
        // Limit rendering frequency for better performance
        let now = Instant::now();
        if force_render || now.duration_since(last_render) >= RENDER_THRESHOLD {
            if let Err(e) = draw_ui(
                stdout,
                &shortcut,
                &snippet,
                current_field,
                cursor_pos,
                current_line,
                editor_mode,
                error_message.as_deref(),
            ) {
                // Try minimal UI if main UI fails
                error_message = Some(format!("UI Error: {}. Using minimal mode.", e));

                if let Err(_) = execute!(
                    stdout,
                    terminal::Clear(ClearType::All),
                    cursor::MoveTo(0, 0),
                    SetForegroundColor(Color::Red),
                    Print(&error_message.clone().unwrap_or_default()),
                    ResetColor,
                    cursor::MoveTo(0, 2),
                    SetForegroundColor(Color::White),
                    Print(format!("Shortcut: {}\n", shortcut)),
                    Print(format!(
                        "Editing {} lines, current: {}\n",
                        snippet.len(),
                        current_line + 1
                    )),
                    Print("Press Ctrl+W to save or Esc to cancel\n"),
                    ResetColor
                ) {
                    // If even the minimal UI fails, return a usable error
                    return Err(ScribeError::Other(
                        "Terminal display error. Try in a larger terminal.".to_string(),
                    ));
                }
            } else {
                error_message = None;
            }
            // Update the render time and reset the force render flag
            last_render = now;
            force_render = false;
        }

        // Handle UI events
        if crossterm::event::poll(Duration::from_millis(16))? {
            match event::read() {
                Ok(Event::Key(KeyEvent {
                    code, modifiers, ..
                })) => {
                    force_render = true; // Force render on key input

                    // Special handling for paste mode
                    if editor_mode == EditorMode::Paste {
                        match code {
                            KeyCode::Esc => {
                                // Cancel paste operation
                                editor_mode = EditorMode::Insert;
                                paste_buffer.clear();
                            }
                            KeyCode::Enter => {
                                // Process paste buffer
                                if !paste_buffer.is_empty() {
                                    process_paste_buffer(
                                        &mut snippet,
                                        &mut current_line,
                                        &mut cursor_pos,
                                        &paste_buffer,
                                    );
                                    paste_buffer.clear();
                                }
                                editor_mode = EditorMode::Insert;
                            }
                            KeyCode::Char(c) => {
                                // Add to paste buffer
                                paste_buffer.push(c);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Normal input handling
                    match code {
                        KeyCode::Tab | KeyCode::Down => {
                            if current_field == 0 {
                                current_field = 1;
                                current_line = 0;
                                cursor_pos = snippet[current_line].len();
                            } else if modifiers.contains(KeyModifiers::SHIFT) {
                                current_field = 0;
                                cursor_pos = shortcut.len();
                            } else if current_line < snippet.len() - 1 {
                                current_line += 1;
                                cursor_pos = snippet[current_line].len().min(cursor_pos);
                            }
                        }
                        KeyCode::BackTab | KeyCode::Up => {
                            if current_field == 1 {
                                if current_line > 0 {
                                    current_line -= 1;
                                    cursor_pos = snippet[current_line].len().min(cursor_pos);
                                } else {
                                    current_field = 0;
                                    cursor_pos = shortcut.len();
                                }
                            }
                        }
                        KeyCode::Esc => {
                            // Check if we're in Insert mode with empty fields for quick exit
                            if editor_mode == EditorMode::Insert
                                && shortcut.is_empty()
                                && (snippet.len() == 1 && snippet[0].is_empty())
                            {
                                return Ok(false); // Return false to indicate cancellation
                            }

                            if editor_mode == EditorMode::Normal {
                                return Ok(snippet_added); // Return based on whether we added a snippet
                            } else {
                                editor_mode = EditorMode::Normal;
                            }
                        }
                        KeyCode::Char('v') if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Enter paste mode
                            editor_mode = EditorMode::Paste;
                            paste_buffer.clear();
                            // Show paste indicator
                            if let Err(_) = execute!(
                                stdout,
                                cursor::MoveTo(0, 0),
                                SetForegroundColor(Color::Green),
                                Print(
                                    "PASTE MODE: Type or paste text, then press Enter to confirm"
                                ),
                                ResetColor
                            ) {
                                // Non-critical error
                            }
                        }

                        _ => {
                            if current_field == 0 {
                                // Shortcut field handling
                                handle_shortcut_input(
                                    &mut shortcut,
                                    &mut cursor_pos,
                                    code,
                                    modifiers,
                                )?;
                                if code == KeyCode::Enter {
                                    current_field = 1;
                                    current_line = 0;
                                    cursor_pos = snippet[current_line].len();
                                }
                            } else {
                                // Snippet field handling with vim-like modes
                                match editor_mode {
                                    EditorMode::Normal => {
                                        handle_normal_mode(
                                            &mut snippet,
                                            &mut cursor_pos,
                                            &mut current_line,
                                            &mut editor_mode,
                                            code,
                                            modifiers,
                                            stdout,
                                            &shortcut,
                                        )?;
                                    }
                                    EditorMode::Insert => {
                                        handle_insert_mode(
                                            &mut snippet,
                                            &mut cursor_pos,
                                            &mut current_line,
                                            &mut editor_mode,
                                            code,
                                            modifiers,
                                            stdout,
                                            &shortcut,
                                        )?;
                                    }
                                    EditorMode::Paste => { /* Handled above */ }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error_message = Some(format!("Input error: {}. Press any key to continue.", e));
                    thread_sleep(1000);
                }
                _ => {}
            }
        }
        if snippet_added {
            return Ok(true);
        }
    }
}

// Process a paste buffer efficiently by handling it all at once
fn process_paste_buffer(
    snippet: &mut Vec<String>,
    current_line: &mut usize,
    cursor_pos: &mut usize,
    buffer: &str,
) {
    // Split the paste buffer by lines
    let lines: Vec<&str> = buffer.split('\n').collect();

    if lines.is_empty() {
        return;
    }

    // Clone the current line to avoid borrowing issues
    let current = snippet[*current_line].clone();

    // Split at cursor position
    let before = &current[..(*cursor_pos).min(current.len())];
    let after = &current[(*cursor_pos).min(current.len())..];

    // Replace current line with first part + first line of paste
    snippet[*current_line] = format!("{}{}", before, lines[0]);
    *cursor_pos = before.len() + lines[0].len();

    // Insert the rest of the lines
    for (i, &line) in lines.iter().enumerate().skip(1) {
        if snippet.len() >= MAX_LINES {
            break;
        }

        let insertion_index = *current_line + i;

        // For the last line of paste, append the remainder of the original line
        if i == lines.len() - 1 {
            let combined_line = format!("{}{}", line, after);
            if insertion_index < snippet.len() {
                snippet.insert(insertion_index, combined_line);
            } else {
                snippet.push(combined_line);
            }
        } else {
            if insertion_index < snippet.len() {
                snippet.insert(insertion_index, line.to_string());
            } else {
                snippet.push(line.to_string());
            }
        }
    }

    // Update current line position
    if lines.len() > 1 {
        *current_line += lines.len() - 1;
    }
}

// Handle shortcut field input
fn handle_shortcut_input(
    shortcut: &mut String,
    cursor_pos: &mut usize,
    code: KeyCode,
    _modifiers: KeyModifiers,
) -> Result<()> {
    match code {
        KeyCode::Enter => {
            // Return true to indicate field change
            return Ok(());
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                let new_pos = find_prev_char_boundary(shortcut, *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);
                shortcut.replace_range(new_pos..*cursor_pos, "");
                *cursor_pos = new_pos;
            }
        }
        KeyCode::Delete => {
            if *cursor_pos < shortcut.len() {
                let next_pos = find_next_char_boundary(shortcut, *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(shortcut.len());
                shortcut.replace_range(*cursor_pos..next_pos, "");
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos = find_prev_char_boundary(shortcut, *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);
            }
        }
        KeyCode::Right => {
            if *cursor_pos < shortcut.len() {
                *cursor_pos = find_next_char_boundary(shortcut, *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(shortcut.len());
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
        }
        KeyCode::End => {
            *cursor_pos = shortcut.len();
        }
        KeyCode::Char(c) => {
            if shortcut.len() < MAX_SHORTCUT_LENGTH {
                shortcut.insert(*cursor_pos, c);
                *cursor_pos += 1;
            }
        }
        _ => {}
    }

    Ok(())
}

// Handle normal mode input (vim-like)
fn handle_normal_mode(
    snippet: &mut Vec<String>,
    cursor_pos: &mut usize,
    current_line: &mut usize,
    editor_mode: &mut EditorMode,
    code: KeyCode,
    modifiers: KeyModifiers,
    stdout: &mut io::Stdout,
    shortcut: &str,
) -> Result<()> {
    match code {
        KeyCode::Char('i') => {
            *editor_mode = EditorMode::Insert;
        }
        KeyCode::Char('a') => {
            if *cursor_pos < snippet[*current_line].len() {
                *cursor_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());
            }
            *editor_mode = EditorMode::Insert;
        }
        KeyCode::Char('A') => {
            *cursor_pos = snippet[*current_line].len();
            *editor_mode = EditorMode::Insert;
        }
        KeyCode::Char('o') => {
            if snippet.len() < MAX_LINES {
                snippet.insert(*current_line + 1, String::new());
                *current_line += 1;
                *cursor_pos = 0;
                *editor_mode = EditorMode::Insert;
            }
        }
        KeyCode::Char('O') => {
            if snippet.len() < MAX_LINES {
                snippet.insert(*current_line, String::new());
                *cursor_pos = 0;
                *editor_mode = EditorMode::Insert;
            }
        }
        KeyCode::Char('h') => {
            if *cursor_pos > 0 {
                *cursor_pos = find_prev_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);
            } else if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].len();
            }
        }
        KeyCode::Char('l') => {
            if *cursor_pos < snippet[*current_line].len() {
                *cursor_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());
            } else if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = 0;
            }
        }
        KeyCode::Char('j') => {
            if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
            }
        }
        KeyCode::Char('k') => {
            if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
            }
        }
        KeyCode::Char('0') => {
            *cursor_pos = 0;
        }
        KeyCode::Char('$') => {
            *cursor_pos = snippet[*current_line].len();
        }
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            if snippet.len() > 1 {
                snippet.remove(*current_line);
                if *current_line >= snippet.len() {
                    *current_line = snippet.len() - 1;
                }
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
            } else {
                snippet[0].clear();
                *cursor_pos = 0;
            }
        }
        KeyCode::Enter => {
            submit_snippet(stdout, shortcut, snippet)?;
            return Ok(());
        }
        _ => {}
    }

    Ok(())
}

// Handle insert mode input
fn handle_insert_mode(
    snippet: &mut Vec<String>,
    cursor_pos: &mut usize,
    current_line: &mut usize,
    editor_mode: &mut EditorMode,
    code: KeyCode,
    modifiers: KeyModifiers,
    stdout: &mut io::Stdout,
    shortcut: &str,
) -> Result<()> {
    match code {
        KeyCode::Esc => {
            // Check if both fields are empty - if so, return false for "canceled"
            if shortcut.is_empty() && (snippet.len() == 1 && snippet[0].is_empty()) {
                *editor_mode = EditorMode::Normal;
            }
            *editor_mode = EditorMode::Normal;
        }
        KeyCode::Enter => {
            if snippet.len() < MAX_LINES {
                // Create a new line by splitting at cursor
                let current = snippet[*current_line].clone();
                let (before, after) = current.split_at((*cursor_pos).min(current.len()));

                snippet[*current_line] = before.to_string();
                snippet.insert(*current_line + 1, after.to_string());
                *current_line += 1;
                *cursor_pos = 0;
            }
        }
        KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
            submit_snippet(stdout, shortcut, snippet)?;
            return Ok(());
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                // Safely delete character before cursor
                let new_pos = find_prev_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);

                snippet[*current_line].replace_range(new_pos..*cursor_pos, "");
                *cursor_pos = new_pos;
            } else if *current_line > 0 {
                // At start of line, merge with previous line
                let content = snippet.remove(*current_line);
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].len();
                snippet[*current_line].push_str(&content);
            }
        }
        KeyCode::Delete => {
            if *cursor_pos < snippet[*current_line].len() {
                // Safely delete character at cursor
                let next_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());

                snippet[*current_line].replace_range(*cursor_pos..next_pos, "");
            } else if *current_line < snippet.len() - 1 {
                // At end of line, merge with next line
                let next = snippet.remove(*current_line + 1);
                snippet[*current_line].push_str(&next);
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos = find_prev_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);
            } else if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].len();
            }
        }
        KeyCode::Right => {
            if *cursor_pos < snippet[*current_line].len() {
                *cursor_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());
            } else if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = 0;
            }
        }
        KeyCode::Up => {
            if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
            }
        }
        KeyCode::Down => {
            if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
        }
        KeyCode::End => {
            *cursor_pos = snippet[*current_line].len();
        }
        KeyCode::Tab => {
            if snippet[*current_line].len() < MAX_LINE_LENGTH - 4 {
                // Insert 4 spaces for tab
                snippet[*current_line].insert_str(*cursor_pos, "    ");
                *cursor_pos += 4;
            }
        }
        KeyCode::Char(c) => {
            if snippet[*current_line].len() < MAX_LINE_LENGTH {
                snippet[*current_line].insert(*cursor_pos, c);
                *cursor_pos += 1;
            }
        }
        _ => {}
    }

    Ok(())
}

// Helper to submit a snippet
fn submit_snippet(stdout: &mut io::Stdout, shortcut: &str, snippet: &[String]) -> Result<bool> {
    if shortcut.is_empty() || snippet.is_empty() || snippet[0].is_empty() {
        show_error_message(stdout, "Both fields must be filled")?;
        thread_sleep(1500);
        return Ok(false); // Return false for "not completed"
    }

    // Join the lines with newlines
    let full_snippet = snippet.join("\n");
    match add_snippet(shortcut.to_string(), full_snippet) {
        Ok(_) => {
            show_success_message(stdout)?;
            // Important: Return true to indicate success
            return Ok(true);
        }
        Err(ScribeError::Other(msg)) if msg.contains("already exists") => {
            show_error_message(stdout, &msg)?;
            thread_sleep(1500);
            return Ok(false);
        }
        Err(e) => return Err(e),
    }
}

fn draw_ui(
    stdout: &mut io::Stdout,
    shortcut: &str,
    snippet: &[String],
    current_field: usize,
    cursor_pos: usize,
    current_line: usize,
    editor_mode: EditorMode,
    error_msg: Option<&str>,
) -> Result<()> {
    // Get terminal size safely
    let (width, height) = match terminal::size() {
        Ok((w, h)) => (w, h),
        Err(e) => {
            return Err(ScribeError::Other(format!(
                "Failed to get terminal size: {}",
                e
            )))
        }
    };

    // Check if terminal is too small
    if width < 40 || height < 15 {
        return Err(ScribeError::Other(format!(
            "Terminal too small. Minimum size: 40x15, current: {}x{}",
            width, height
        )));
    }

    let panel_width = width.saturating_sub(10);
    let panel_height = height.saturating_sub(8); // Larger panel for multiline content
    let start_x = 5;
    let start_y = 4;

    // Clear the screen once at the beginning
    if let Err(e) = execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::Hide // Hide cursor during drawing to reduce flicker
    ) {
        return Err(ScribeError::Other(format!("Failed to clear screen: {}", e)));
    }
    let title = match editor_mode {
        EditorMode::Paste => " Paste Mode - Enter to confirm, Esc to cancel ",
        _ => " Add New Snippet ",
    };

    let title_x = start_x + (panel_width - title.len() as u16) / 2;
    if let Err(e) = execute!(
        stdout,
        cursor::Hide, // Hide cursor during drawing to reduce flicker
        cursor::MoveTo(title_x, start_y - 1),
        SetForegroundColor(if editor_mode == EditorMode::Paste {
            Color::Green
        } else {
            Color::Cyan
        }),
        Print(title),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!("Failed to draw title: {}", e)));
    }

    // Draw the box in a single batch
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(start_x, start_y),
        SetForegroundColor(Color::Blue),
        Print("╭"),
        Print("─".repeat((panel_width - 2) as usize)),
        Print("╮")
    ) {
        return Err(ScribeError::Other(format!("Failed to draw box top: {}", e)));
    }

    // Side borders
    for i in 1..panel_height - 1 {
        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(start_x, start_y + i),
            Print("│"),
            cursor::MoveTo(start_x + panel_width - 1, start_y + i),
            Print("│")
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw box sides at row {}: {}",
                i, e
            )));
        }
    }

    // Execute all the box drawing commands in one go
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(start_x, start_y + panel_height - 1),
        Print("╰"),
        Print("─".repeat((panel_width - 2) as usize)),
        Print("╯"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw box bottom: {}",
            e
        )));
    }

    // Draw shortcut field (skip if in paste mode)
    if editor_mode != EditorMode::Paste {
        let field_x = start_x + 3;
        if let Err(e) = draw_field(
            stdout,
            field_x,
            start_y + 2,
            panel_width - 6,
            "Shortcut:",
            shortcut,
            current_field == 0,
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw shortcut field: {}",
                e
            )));
        }
    }

    // Clear the multiline field area before redrawing
    let field_x = start_x + 3;
    if let Err(e) = draw_multiline_field(
        stdout,
        field_x,
        start_y + 6,
        panel_width - 6,
        panel_height - 10, // Allow room for the multiline field
        "Snippet:",
        snippet,
        current_field == 1,
        current_line,
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw snippet field: {}",
            e
        )));
    }

    // Draw help text
    let normal_help =
        "i/a: Insert | o/O: New line | h/j/k/l: Navigate | Ctrl+d: Delete line | Ctrl+w: Submit";
    let insert_help =
        "Esc: Cancel | Enter: New line | Arrows: Navigate | Ctrl+v: Paste | Ctrl+w: Submit";
    let paste_help = "Enter: Confirm paste | Esc: Cancel | Type or paste text";

    let help_text = match editor_mode {
        EditorMode::Normal => normal_help,
        EditorMode::Insert => {
            if current_field == 0 {
                "Tab: Next field | Esc: Cancel"
            } else {
                insert_help
            }
        }
        EditorMode::Paste => paste_help,
    };

    // Center the help text
    let help_x = if help_text.len() as u16 <= panel_width - 4 {
        start_x + (panel_width - help_text.len() as u16) / 2
    } else {
        start_x + 2
    };
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(help_x, start_y + panel_height - 2),
        SetForegroundColor(Color::DarkGrey),
        Print(if help_text.len() as u16 <= panel_width - 4 {
            help_text
        } else {
            &help_text[0..(panel_width - 7) as usize]
        }),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw help text: {}",
            e
        )));
    }

    // Show editor mode if in snippet field
    if current_field == 1 && editor_mode != EditorMode::Paste {
        let mode_text = match editor_mode {
            EditorMode::Normal => "-- NORMAL --",
            EditorMode::Insert => "-- INSERT --",
            EditorMode::Paste => "-- PASTE --",
        };

        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(field_x, start_y + 5),
            SetForegroundColor(if matches!(editor_mode, EditorMode::Normal) {
                Color::Blue
            } else {
                Color::Green
            }),
            Print(mode_text),
            ResetColor
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw mode text: {}",
                e
            )));
        }
    }

    // If there's an error message, display it
    if let Some(msg) = error_msg {
        let err_x = start_x + 2;
        let err_y = start_y + panel_height;

        // Truncate message if needed
        let display_msg = if msg.len() > (panel_width - 4) as usize {
            &msg[0..(panel_width - 7) as usize]
        } else {
            msg
        };

        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(err_x, err_y),
            SetForegroundColor(Color::Red),
            Print(display_msg),
            ResetColor
        ) {
            // If we can't even print the error, just log it and continue
            eprintln!("Failed to show error: {}", e);
        }
    }

    // Position cursor - with robust error handling
    let cursor_result = if editor_mode == EditorMode::Paste {
        // In paste mode, position cursor at top of screen where paste text appears
        execute!(stdout, cursor::MoveTo(0, 1), cursor::Show)
    } else if current_field == 0 {
        let visible_cursor_pos = cursor_pos.min(panel_width as usize - 9) as u16;
        execute!(
            stdout,
            cursor::MoveTo(field_x + 1 + visible_cursor_pos, start_y + 3),
            cursor::Show
        )
    } else {
        // Position cursor in multiline field with scroll offset consideration
        let visible_area_height = (panel_height - 10) as usize;
        let scroll_offset = if current_line >= visible_area_height {
            current_line - visible_area_height + 1
        } else {
            0
        };

        let visible_line_idx = current_line - scroll_offset;
        let visible_cursor_pos = cursor_pos.min(panel_width as usize - 9) as u16;
        execute!(
            stdout,
            cursor::MoveTo(
                field_x + 1 + visible_cursor_pos,
                start_y + 7 + visible_line_idx as u16
            ),
            cursor::Show
        )
    };

    if let Err(e) = cursor_result {
        return Err(ScribeError::Other(format!(
            "Failed to position cursor: {}",
            e
        )));
    }

    // Flush output
    if let Err(e) = stdout.flush() {
        return Err(ScribeError::Other(format!("Failed to flush output: {}", e)));
    }

    Ok(())
}

fn draw_box(stdout: &mut io::Stdout, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
    // Draw each part separately with error checking

    // Top border
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Blue),
        Print("╭"),
        Print("─".repeat((width - 2) as usize)),
        Print("╮")
    ) {
        return Err(ScribeError::Other(format!("Failed to draw box top: {}", e)));
    }

    // Side borders
    for i in 1..height - 1 {
        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(x, y + i),
            Print("│"),
            cursor::MoveTo(x + width - 1, y + i),
            Print("│")
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw box sides at row {}: {}",
                i, e
            )));
        }
    }

    // Bottom border
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + height - 1),
        Print("╰"),
        Print("─".repeat((width - 2) as usize)),
        Print("╯"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw box bottom: {}",
            e
        )));
    }

    Ok(())
}

fn draw_field(
    stdout: &mut io::Stdout,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    value: &str,
    active: bool,
) -> Result<()> {
    // Draw label
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Yellow),
        Print(label),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw field label: {}",
            e
        )));
    }

    // Draw field box top
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + 1),
        SetForegroundColor(Color::Blue),
        Print("┌"),
        Print("─".repeat((width - 2) as usize)),
        Print("┐"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw field box top: {}",
            e
        )));
    }

    let bg_color = if active {
        Color::DarkBlue
    } else {
        Color::Black
    };
    let fg_color = if active { Color::White } else { Color::Grey };

    // Safely process value for display
    let visible_text = safe_truncate_string(value, width as usize - 4, true);

    // Draw field content
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + 2),
        SetForegroundColor(Color::Blue),
        Print("│"),
        SetBackgroundColor(bg_color),
        SetForegroundColor(fg_color),
        Print(" "),
        Print(&visible_text),
        Print(" ".repeat((width as usize - 3 - visible_text.chars().count()).max(0))),
        ResetColor,
        SetForegroundColor(Color::Blue),
        Print("│"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw field content: {}",
            e
        )));
    }

    // Draw field box bottom
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + 3),
        SetForegroundColor(Color::Blue),
        Print("└"),
        Print("─".repeat((width - 2) as usize)),
        Print("┘"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw field box bottom: {}",
            e
        )));
    }

    Ok(())
}

fn draw_multiline_field(
    stdout: &mut io::Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    label: &str,
    lines: &[String],
    active: bool,
    current_line: usize,
) -> Result<()> {
    // Draw label
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Yellow),
        Print(label),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw multiline field label: {}",
            e
        )));
    }

    // Draw field box top
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + 1),
        SetForegroundColor(Color::Blue),
        Print("┌"),
        Print("─".repeat((width - 2) as usize)),
        Print("┐"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw multiline field box top: {}",
            e
        )));
    }

    // Calculate appropriate scroll position to keep current line visible
    let visible_area_height = height as usize;
    let scroll_offset = if current_line >= visible_area_height {
        current_line - visible_area_height + 1
    } else {
        0
    };

    // Show line numbers and scroll indicator if there are multiple lines
    if lines.len() > 1 {
        let scroll_info = format!(" {}/{} ", current_line + 1, lines.len());
        let info_x = x + width - scroll_info.len() as u16 - 2;

        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(info_x, y + 1),
            SetForegroundColor(Color::Yellow),
            Print(scroll_info),
            ResetColor
        ) {
            // Non-critical error - just continue without scroll info
            eprintln!("Failed to draw scroll info: {}", e);
        }
    }

    // Color settings
    let bg_color = if active { Color::Blue } else { Color::Black };
    let fg_color = if active { Color::White } else { Color::Grey };

    // Draw each visible line
    let max_visible_lines = height as usize;
    let end_line = (scroll_offset + max_visible_lines).min(lines.len());

    for i in 0..(end_line - scroll_offset) {
        let line_idx = i + scroll_offset;
        let line = &lines[line_idx];

        // Safely process line for display
        let visible_text = safe_truncate_string(line, width as usize - 4, true);

        let is_current = line_idx == current_line && active;
        let line_bg = if is_current { bg_color } else { Color::Black };
        let line_fg = if is_current { Color::White } else { fg_color };

        // Padding to fill the line
        let padding_length = (width as usize - 3 - visible_text.chars().count()).max(0);

        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(x, y + 2 + i as u16),
            SetForegroundColor(Color::Blue),
            Print("│"),
            SetBackgroundColor(line_bg),
            SetForegroundColor(line_fg),
            Print(" "),
            Print(&visible_text),
            Print(" ".repeat(padding_length)),
            ResetColor,
            SetForegroundColor(Color::Blue),
            Print("│"),
            ResetColor
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw line {} of multiline field: {}",
                i, e
            )));
        }
    }

    // Fill remaining lines with empty space
    for i in (end_line - scroll_offset)..max_visible_lines {
        if let Err(e) = execute!(
            stdout,
            cursor::MoveTo(x, y + 2 + i as u16),
            SetForegroundColor(Color::Blue),
            Print("│"),
            Print(" ".repeat((width - 2) as usize)),
            Print("│"),
            ResetColor
        ) {
            return Err(ScribeError::Other(format!(
                "Failed to draw empty line {} of multiline field: {}",
                i, e
            )));
        }
    }

    // Draw field box bottom
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + 2 + height),
        SetForegroundColor(Color::Blue),
        Print("└"),
        Print("─".repeat((width - 2) as usize)),
        Print("┘"),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to draw multiline field box bottom: {}",
            e
        )));
    }

    Ok(())
}

// Helper function to find previous character boundary
fn find_prev_char_boundary(s: &str, pos: usize) -> Option<usize> {
    if pos == 0 || pos > s.len() {
        return None;
    }

    // Start from position and search backward
    for i in (0..pos).rev() {
        if s.is_char_boundary(i) {
            return Some(i);
        }
    }

    Some(0) // Return beginning of string if no boundary found
}

// Helper function to find next character boundary
fn find_next_char_boundary(s: &str, pos: usize) -> Option<usize> {
    if pos >= s.len() {
        return None;
    }

    // Start from position and search forward
    for i in (pos + 1)..=s.len() {
        if s.is_char_boundary(i) {
            return Some(i);
        }
    }

    Some(s.len()) // Return end of string if no boundary found
}

// Safe string truncation that respects UTF-8 boundaries
fn safe_truncate_string(s: &str, max_width: usize, add_ellipsis: bool) -> String {
    if s.is_empty() || max_width == 0 {
        return String::new();
    }

    // If the string is shorter than max_width, return it as is
    if s.chars().count() <= max_width {
        return s.to_string();
    }

    // Otherwise truncate at character boundaries
    let mut result = String::with_capacity(max_width + 3); // +3 for possible ellipsis
    let mut count = 0;
    let actual_max = if add_ellipsis {
        max_width - 3
    } else {
        max_width
    };

    for c in s.chars() {
        if count >= actual_max {
            break;
        }
        result.push(c);
        count += 1;
    }

    if add_ellipsis && count < s.chars().count() {
        result.push_str("...");
    }

    result
}

fn show_success_message(stdout: &mut io::Stdout) -> Result<()> {
    // Try to get terminal size, with fallback
    let (width, height) = terminal::size().unwrap_or((80, 24));
    let message = "✓ Snippet added successfully!";
    let x = (width.saturating_sub(message.len() as u16)) / 2;
    let y = height / 2;

    // Multiple commands with individual error handling
    if let Err(e) = execute!(stdout, terminal::Clear(ClearType::All)) {
        return Err(ScribeError::Other(format!("Failed to clear screen: {}", e)));
    }

    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Green),
        Print(message),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to show success message: {}",
            e
        )));
    }

    // Add a hint about returning to the snippet manager
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x - 5, y + 2),
        SetForegroundColor(Color::White),
        Print("Returning to snippet manager..."),
        ResetColor
    ) {
        // Non-critical error, can continue
        eprintln!("Failed to show return message: {}", e);
    }

    if let Err(e) = stdout.flush() {
        return Err(ScribeError::Other(format!("Failed to flush output: {}", e)));
    }

    thread_sleep(1000); // Reduced to 1 second for faster transition
    Ok(())
}

fn show_error_message(stdout: &mut io::Stdout, message: &str) -> Result<()> {
    // Try to get terminal size, with fallback
    let (width, height) = terminal::size().unwrap_or((80, 24));

    // Truncate message if too long
    let display_msg = safe_truncate_string(message, width as usize - 10, true);

    let x = (width.saturating_sub(display_msg.len() as u16)) / 2;
    let y = height - 3;

    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Red),
        Print(display_msg),
        ResetColor
    ) {
        return Err(ScribeError::Other(format!(
            "Failed to show error message: {}",
            e
        )));
    }

    if let Err(e) = stdout.flush() {
        return Err(ScribeError::Other(format!("Failed to flush output: {}", e)));
    }

    Ok(())
}

fn thread_sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
