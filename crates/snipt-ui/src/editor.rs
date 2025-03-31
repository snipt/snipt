use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use snipt_core::{add_snippet, Result, SniptError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{
    io::{self, stdout, Write},
    thread,
};

// Constants
const MAX_LINES: usize = 10000;
const MAX_LINE_LENGTH: usize = 5000;
const MAX_SHORTCUT_LENGTH: usize = 50;

#[derive(PartialEq, Copy, Clone)]
enum EditorMode {
    Normal,
    Insert,
    Paste, // New mode for pasting large text
}

pub enum AddResult {
    Added,
    Cancelled,
    Error(SniptError),
}

pub fn interactive_add() -> AddResult {
    // Setup terminal with error handling
    if let Err(e) = terminal::enable_raw_mode() {
        return AddResult::Error(SniptError::Other(format!(
            "Failed to enable raw mode: {}",
            e
        )));
    }

    let mut stdout = stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        terminal::disable_raw_mode().ok();
        return AddResult::Error(SniptError::Other(format!(
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
    let mut snippet_added = false;

    // For paste handling
    let mut paste_buffer = String::new();

    // For performance optimization
    let mut last_render = Instant::now();
    let mut force_render = true;

    // Initial draw to set up the UI
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
        error_message = Some(format!("UI Error: {}. Using minimal mode.", e));
    }

    // Much shorter rendering interval - decreased to prevent flickering
    const RENDER_THRESHOLD: Duration = Duration::from_millis(16); // ~60fps (smoother)

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
                    return Err(SniptError::Other(
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

        // Handle UI events with a very short timeout to remain responsive
        if crossterm::event::poll(Duration::from_millis(1))? {
            match event::read() {
                Ok(Event::Key(KeyEvent {
                    code, modifiers, ..
                })) => {
                    // Only force render when the state actually changes
                    let mut state_changed = false;

                    // Special handling for paste mode
                    if editor_mode == EditorMode::Paste {
                        match code {
                            KeyCode::Esc => {
                                // Cancel paste operation
                                paste_buffer.clear();
                                return Ok(false);
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
                                state_changed = true;
                            }
                            KeyCode::Char(c) => {
                                // Add to paste buffer
                                paste_buffer.push(c);
                                // Don't redraw for each character in paste mode
                            }
                            _ => {}
                        }
                        if state_changed {
                            force_render = true;
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
                                state_changed = true;
                            } else if modifiers.contains(KeyModifiers::SHIFT) {
                                current_field = 0;
                                cursor_pos = shortcut.len();
                                state_changed = true;
                            } else if current_line < snippet.len() - 1 {
                                current_line += 1;
                                cursor_pos = snippet[current_line].len().min(cursor_pos);
                                state_changed = true;
                            }
                        }
                        KeyCode::BackTab | KeyCode::Up => {
                            if current_field == 1 {
                                if current_line > 0 {
                                    current_line -= 1;
                                    cursor_pos = snippet[current_line].len().min(cursor_pos);
                                    state_changed = true;
                                } else {
                                    current_field = 0;
                                    cursor_pos = shortcut.len();
                                    state_changed = true;
                                }
                            }
                        }
                        KeyCode::Esc => {
                            // Check for empty fields and return false to indicate cancel
                            if shortcut.is_empty() && (snippet.len() == 1 && snippet[0].is_empty())
                            {
                                return Ok(false);
                            }

                            if editor_mode == EditorMode::Normal {
                                return Ok(snippet_added);
                            } else {
                                editor_mode = EditorMode::Normal;
                                state_changed = true;
                            }
                        }
                        KeyCode::Char('v') if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Enter paste mode
                            editor_mode = EditorMode::Paste;
                            paste_buffer.clear();
                            state_changed = true;
                        }

                        _ => {
                            if current_field == 0 {
                                // Shortcut field handling
                                if handle_shortcut_input(
                                    &mut shortcut,
                                    &mut cursor_pos,
                                    code,
                                    modifiers,
                                )? {
                                    state_changed = true;
                                }
                                if code == KeyCode::Enter {
                                    current_field = 1;
                                    current_line = 0;
                                    cursor_pos = snippet[current_line].len();
                                    state_changed = true;
                                }
                            } else {
                                // Snippet field handling with vim-like modes
                                match editor_mode {
                                    EditorMode::Normal => {
                                        if handle_normal_mode(
                                            &mut snippet,
                                            &mut cursor_pos,
                                            &mut current_line,
                                            &mut editor_mode,
                                            code,
                                            modifiers,
                                            stdout,
                                            &shortcut,
                                            &mut snippet_added,
                                        )? {
                                            state_changed = true;
                                        }
                                    }
                                    EditorMode::Insert => {
                                        if handle_insert_mode(
                                            &mut snippet,
                                            &mut cursor_pos,
                                            &mut current_line,
                                            &mut editor_mode,
                                            code,
                                            modifiers,
                                            stdout,
                                            &shortcut,
                                            &mut snippet_added,
                                        )? {
                                            state_changed = true;
                                        }
                                    }
                                    EditorMode::Paste => { /* Handled above */ }
                                }
                            }
                        }
                    }

                    // Only redraw if state actually changed
                    if state_changed {
                        force_render = true;
                    }
                }
                Err(e) => {
                    error_message = Some(format!("Input error: {}. Press any key to continue.", e));
                    thread_sleep(1000);
                    force_render = true;
                }
                _ => {}
            }
        } else {
            // Small sleep to prevent CPU usage when idle
            thread::sleep(Duration::from_millis(1));
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
) -> Result<bool> {
    let mut state_changed = false;
    match code {
        KeyCode::Enter => {
            // Return true to indicate field change
            return Ok(true);
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                // Convert to chars for proper UTF-8 handling
                let mut chars: Vec<char> = shortcut.chars().collect();
                let cursor_char_pos = (*cursor_pos).min(chars.len());

                if cursor_char_pos > 0 {
                    // Remove character before cursor
                    chars.remove(cursor_char_pos - 1);
                    *shortcut = chars.into_iter().collect();
                    *cursor_pos -= 1;
                    state_changed = true;
                }
            }
        }
        KeyCode::Delete => {
            // Convert to chars for proper UTF-8 handling
            let mut chars: Vec<char> = shortcut.chars().collect();
            let cursor_char_pos = (*cursor_pos).min(chars.len());

            if cursor_char_pos < chars.len() {
                // Remove character at cursor
                chars.remove(cursor_char_pos);
                *shortcut = chars.into_iter().collect();
                state_changed = true;
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos -= 1;
                state_changed = true;
            }
        }
        KeyCode::Right => {
            let char_count = shortcut.chars().count();
            if *cursor_pos < char_count {
                *cursor_pos += 1;
                state_changed = true;
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
            state_changed = true;
        }
        KeyCode::End => {
            *cursor_pos = shortcut.chars().count(); // Count chars, not bytes
            state_changed = true;
        }
        KeyCode::Char(c) => {
            if shortcut.len() < MAX_SHORTCUT_LENGTH {
                // Convert to chars for proper UTF-8 handling
                let mut chars: Vec<char> = shortcut.chars().collect();
                let cursor_char_pos = (*cursor_pos).min(chars.len());

                // Insert character at cursor position
                chars.insert(cursor_char_pos, c);

                // Convert back to string
                *shortcut = chars.into_iter().collect();
                *cursor_pos += 1;
                state_changed = true;
            }
        }
        _ => {}
    }

    Ok(state_changed)
}

fn handle_normal_mode(
    snippet: &mut Vec<String>,
    cursor_pos: &mut usize,
    current_line: &mut usize,
    editor_mode: &mut EditorMode,
    code: KeyCode,
    modifiers: KeyModifiers,
    stdout: &mut io::Stdout,
    shortcut: &str,
    snippet_added: &mut bool,
) -> Result<bool> {
    let mut state_changed = false;
    match code {
        KeyCode::Char('i') => {
            *editor_mode = EditorMode::Insert;
            state_changed = true;
        }
        KeyCode::Char('a') => {
            if *cursor_pos < snippet[*current_line].len() {
                *cursor_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());
            }
            *editor_mode = EditorMode::Insert;
            state_changed = true;
        }
        KeyCode::Char('A') => {
            *cursor_pos = snippet[*current_line].len();
            *editor_mode = EditorMode::Insert;
            state_changed = true;
        }
        KeyCode::Char('o') => {
            if snippet.len() < MAX_LINES {
                snippet.insert(*current_line + 1, String::new());
                *current_line += 1;
                *cursor_pos = 0;
                *editor_mode = EditorMode::Insert;
                state_changed = true;
            }
        }
        KeyCode::Char('O') => {
            if snippet.len() < MAX_LINES {
                snippet.insert(*current_line, String::new());
                *cursor_pos = 0;
                *editor_mode = EditorMode::Insert;
                state_changed = true;
            }
        }
        KeyCode::Char('h') => {
            if *cursor_pos > 0 {
                *cursor_pos = find_prev_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos - 1)
                    .min(*cursor_pos);
                state_changed = true;
            } else if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].len();
                state_changed = true;
            }
        }
        KeyCode::Char('l') => {
            if *cursor_pos < snippet[*current_line].len() {
                *cursor_pos = find_next_char_boundary(&snippet[*current_line], *cursor_pos)
                    .unwrap_or(*cursor_pos + 1)
                    .min(snippet[*current_line].len());
                state_changed = true;
            } else if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = 0;
                state_changed = true;
            }
        }
        KeyCode::Char('j') => {
            if *current_line < snippet.len() - 1 {
                *current_line += 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
                state_changed = true;
            }
        }
        KeyCode::Char('k') => {
            if *current_line > 0 {
                *current_line -= 1;
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
                state_changed = true;
            }
        }
        KeyCode::Char('0') => {
            *cursor_pos = 0;
            state_changed = true;
        }
        KeyCode::Char('$') => {
            *cursor_pos = snippet[*current_line].len();
            state_changed = true;
        }
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            if snippet.len() > 1 {
                snippet.remove(*current_line);
                if *current_line >= snippet.len() {
                    *current_line = snippet.len() - 1;
                }
                *cursor_pos = (*cursor_pos).min(snippet[*current_line].len());
                state_changed = true;
            } else {
                snippet[0].clear();
                *cursor_pos = 0;
                state_changed = true;
            }
        }
        KeyCode::Enter => {
            if let Ok(added) = submit_snippet(stdout, shortcut, snippet) {
                *snippet_added = added;
            }
            return Ok(true);
        }
        _ => {}
    }

    Ok(state_changed)
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
    snippet_added: &mut bool,
) -> Result<bool> {
    let mut state_changed = false;
    match code {
        KeyCode::Esc => {
            // Check if both fields are empty - if so, return false for "canceled"
            if shortcut.is_empty() && (snippet.len() == 1 && snippet[0].is_empty()) {
                *snippet_added = false;
                return Ok(true);
            }
            *editor_mode = EditorMode::Normal;
            state_changed = true;
        }
        KeyCode::Enter => {
            if snippet.len() < MAX_LINES {
                // Create a new line by splitting at cursor
                let current = &snippet[*current_line];

                // Convert to char array to handle UTF-8 correctly
                let chars: Vec<char> = current.chars().collect();
                let cursor_char_pos = (*cursor_pos).min(chars.len());

                // Split string at character position
                let before: String = chars[..cursor_char_pos].iter().collect();
                let after: String = chars[cursor_char_pos..].iter().collect();

                snippet[*current_line] = before;
                snippet.insert(*current_line + 1, after);
                *current_line += 1;
                *cursor_pos = 0;
                state_changed = true;
            }
        }
        KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
            if let Ok(added) = submit_snippet(stdout, shortcut, snippet) {
                *snippet_added = added;
            }
            return Ok(true);
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                // Convert to chars for proper UTF-8 handling
                let mut chars: Vec<char> = snippet[*current_line].chars().collect();
                let cursor_char_pos = (*cursor_pos).min(chars.len());

                if cursor_char_pos > 0 {
                    // Remove character before cursor
                    chars.remove(cursor_char_pos - 1);
                    snippet[*current_line] = chars.into_iter().collect();
                    *cursor_pos -= 1;
                    state_changed = true;
                }
            } else if *current_line > 0 {
                // At start of line, merge with previous line
                let content = snippet.remove(*current_line);
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].chars().count(); // Important: count chars, not bytes
                snippet[*current_line].push_str(&content);
                state_changed = true;
            }
        }
        KeyCode::Delete => {
            // Convert to chars for proper UTF-8 handling
            let mut chars: Vec<char> = snippet[*current_line].chars().collect();
            let cursor_char_pos = (*cursor_pos).min(chars.len());

            if cursor_char_pos < chars.len() {
                // Remove character at cursor
                chars.remove(cursor_char_pos);
                snippet[*current_line] = chars.into_iter().collect();
                state_changed = true;
            } else if *current_line < snippet.len() - 1 {
                // At end of line, merge with next line
                let next = snippet.remove(*current_line + 1);
                snippet[*current_line].push_str(&next);
                state_changed = true;
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos -= 1;
                state_changed = true;
            } else if *current_line > 0 {
                // Move to end of previous line
                *current_line -= 1;
                *cursor_pos = snippet[*current_line].chars().count(); // Count chars, not bytes
                state_changed = true;
            }
        }
        KeyCode::Right => {
            let char_count = snippet[*current_line].chars().count();
            if *cursor_pos < char_count {
                *cursor_pos += 1;
                state_changed = true;
            } else if *current_line < snippet.len() - 1 {
                // Move to start of next line
                *current_line += 1;
                *cursor_pos = 0;
                state_changed = true;
            }
        }
        KeyCode::Up => {
            if *current_line > 0 {
                *current_line -= 1;
                let char_count = snippet[*current_line].chars().count();
                *cursor_pos = (*cursor_pos).min(char_count);
                state_changed = true;
            }
        }
        KeyCode::Down => {
            if *current_line < snippet.len() - 1 {
                *current_line += 1;
                let char_count = snippet[*current_line].chars().count();
                *cursor_pos = (*cursor_pos).min(char_count);
                state_changed = true;
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
            state_changed = true;
        }
        KeyCode::End => {
            *cursor_pos = snippet[*current_line].chars().count(); // Count chars, not bytes
            state_changed = true;
        }
        KeyCode::Tab => {
            // Insert 4 spaces for tab
            if snippet[*current_line].len() < MAX_LINE_LENGTH - 4 {
                for _ in 0..4 {
                    // Convert to chars
                    let mut chars: Vec<char> = snippet[*current_line].chars().collect();
                    let cursor_char_pos = (*cursor_pos).min(chars.len());

                    // Insert space
                    chars.insert(cursor_char_pos, ' ');

                    // Convert back to string
                    snippet[*current_line] = chars.into_iter().collect();
                    *cursor_pos += 1;
                    state_changed = true;
                }
            }
        }
        KeyCode::Char(c) => {
            if snippet[*current_line].len() < MAX_LINE_LENGTH {
                // Convert string to chars for proper UTF-8 handling
                let mut chars: Vec<char> = snippet[*current_line].chars().collect();
                let cursor_char_pos = (*cursor_pos).min(chars.len());

                // Insert character at cursor position
                chars.insert(cursor_char_pos, c);

                // Convert back to string
                snippet[*current_line] = chars.into_iter().collect();
                *cursor_pos += 1;
                state_changed = true;
            }
        }
        _ => {}
    }

    Ok(state_changed)
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
        Err(SniptError::Other(msg)) if msg.contains("already exists") => {
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
    // Use a static AtomicBool for tracking first draw
    static FIRST_DRAW: AtomicBool = AtomicBool::new(true);

    // Get terminal size safely
    let (width, height) = match terminal::size() {
        Ok((w, h)) => (w, h),
        Err(e) => {
            return Err(SniptError::Other(format!(
                "Failed to get terminal size: {}",
                e
            )))
        }
    };

    // Check if terminal is too small
    if width < 40 || height < 15 {
        return Err(SniptError::Other(format!(
            "Terminal too small. Minimum size: 40x15, current: {}x{}",
            width, height
        )));
    }

    // Calculate layout sizes using golden ratio-inspired proportions
    let panel_width = width.saturating_sub(8).max(40);
    let panel_height = height.saturating_sub(6).max(15);
    let start_x = (width - panel_width) / 2; // Center horizontally
    let start_y = (height - panel_height) / 2; // Center vertically

    // Clear the screen only on first draw - using atomic compare_exchange for thread safety
    if FIRST_DRAW
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        if let Err(e) = execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::Hide // Hide cursor during drawing to reduce flicker
        ) {
            return Err(SniptError::Other(format!("Failed to clear screen: {}", e)));
        }
    }

    // Calculate title based on current mode
    let title = match editor_mode {
        EditorMode::Paste => " ✏️  Paste Mode - Enter to confirm ",
        EditorMode::Normal => " ✏️  Add New Snippet - Normal Mode ",
        EditorMode::Insert => " ✏️  Add New Snippet - Insert Mode ",
    };

    let title_x = start_x + (panel_width - title.len() as u16) / 2;

    // Draw the title with better styling
    if let Err(e) = execute!(
        stdout,
        cursor::Hide,
        cursor::MoveTo(title_x, start_y - 1),
        SetForegroundColor(if editor_mode == EditorMode::Paste {
            Color::Green
        } else if editor_mode == EditorMode::Normal {
            Color::Blue
        } else {
            Color::Cyan
        }),
        SetBackgroundColor(Color::Black),
        Print(title),
        ResetColor
    ) {
        return Err(SniptError::Other(format!("Failed to draw title: {}", e)));
    }

    // Draw the outer box with rounded corners for a nicer appearance
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(start_x, start_y),
        SetForegroundColor(Color::Cyan),
        Print("╭"),
        Print("─".repeat((panel_width - 2) as usize)),
        Print("╮")
    ) {
        return Err(SniptError::Other(format!("Failed to draw box top: {}", e)));
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
            return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
            "Failed to draw box bottom: {}",
            e
        )));
    }

    // Add app header/brand (new)
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(start_x + 3, start_y + 1),
        SetForegroundColor(Color::Magenta),
        Print("snipt"),
        SetForegroundColor(Color::DarkGrey),
        Print(" - Text Expansion Tool"),
        ResetColor
    ) {
        return Err(SniptError::Other(format!("Failed to draw header: {}", e)));
    }

    // Draw horizontal separator under header
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(start_x + 1, start_y + 2),
        SetForegroundColor(Color::DarkGrey),
        Print("─".repeat((panel_width - 3) as usize)),
        ResetColor
    ) {
        return Err(SniptError::Other(format!(
            "Failed to draw separator: {}",
            e
        )));
    }

    // Draw shortcut field with improved style
    let field_x = start_x + 3;
    if let Err(e) = draw_field(
        stdout,
        field_x,
        start_y + 4,
        panel_width - 8,
        "Shortcut:",
        shortcut,
        current_field == 0,
    ) {
        return Err(SniptError::Other(format!(
            "Failed to draw shortcut field: {}",
            e
        )));
    }

    // Draw the multiline field with improved style
    let field_x = start_x + 3;
    if let Err(e) = draw_multiline_field(
        stdout,
        field_x,
        start_y + 8,
        panel_width - 6,
        panel_height - 14, // Adjust for better proportions
        "Snippet:",
        snippet,
        current_field == 1,
        current_line,
    ) {
        return Err(SniptError::Other(format!(
            "Failed to draw snippet field: {}",
            e
        )));
    }
    let help_text = match editor_mode {
        EditorMode::Normal => {
            "i/a: Insert | o/O: New line | h/j/k/l: Navigate | Ctrl+d: Delete line | Enter: Submit"
        }
        EditorMode::Insert => {
            if current_field == 0 {
                "Tab: Next field | Enter: Next field | Esc: Cancel"
            } else {
                "Esc: Normal mode | Enter: New line | Arrows: Navigate | Ctrl+v: Paste | Ctrl+w: Submit"
            }
        }
        EditorMode::Paste => "Enter: Confirm paste | Esc: Cancel | Type or paste text",
    };

    // Add a distinctive button-like bottom bar for key actions
    let buttons_line = match editor_mode {
        EditorMode::Insert if current_field == 1 => {
            "[ Ctrl+W: Save ] [ Tab: Indent ] [ Esc: Normal Mode ]"
        }
        EditorMode::Normal => "[ Enter: Submit ] [ i: Insert Mode ] [ Esc: Cancel ]",
        _ => "[ Ctrl+W: Save ] [ Esc: Cancel ]",
    };

    // Center the button bar
    let buttons_x = start_x + (panel_width - buttons_line.len() as u16) / 2;

    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(buttons_x, start_y + panel_height - 3),
        SetForegroundColor(Color::White),
        SetBackgroundColor(Color::DarkBlue),
        Print(buttons_line),
        ResetColor
    ) {
        return Err(SniptError::Other(format!("Failed to draw buttons: {}", e)));
    }

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
        return Err(SniptError::Other(format!(
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
            cursor::MoveTo(field_x, start_y + 7),
            SetForegroundColor(if matches!(editor_mode, EditorMode::Normal) {
                Color::Blue
            } else {
                Color::Green
            }),
            Print(mode_text),
            ResetColor
        ) {
            return Err(SniptError::Other(format!(
                "Failed to draw mode text: {}",
                e
            )));
        }
    }

    // If there's an error message, display it with a red background for visibility
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
            SetForegroundColor(Color::White),
            SetBackgroundColor(Color::Red),
            Print(format!(" {} ", display_msg)),
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
            cursor::MoveTo(field_x + 1 + visible_cursor_pos, start_y + 5),
            cursor::Show
        )
    } else {
        // Position cursor in multiline field with scroll offset consideration
        let visible_area_height = (panel_height - 14) as usize;
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
                start_y + 9 + visible_line_idx as u16
            ),
            cursor::Show
        )
    };

    if let Err(e) = cursor_result {
        return Err(SniptError::Other(format!(
            "Failed to position cursor: {}",
            e
        )));
    }

    // Flush output
    if let Err(e) = stdout.flush() {
        return Err(SniptError::Other(format!("Failed to flush output: {}", e)));
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
        return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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
    let bg_color = if active {
        Color::DarkBlue
    } else {
        Color::Black
    };
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
            return Err(SniptError::Other(format!(
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
            return Err(SniptError::Other(format!(
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
        return Err(SniptError::Other(format!(
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

    // Find the previous valid UTF-8 character boundary
    let mut idx = pos;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }

    // Keep going back to find the actual previous character
    if idx > 0 {
        let mut prev_idx = idx - 1;
        while prev_idx > 0 && !s.is_char_boundary(prev_idx) {
            prev_idx -= 1;
        }
        Some(prev_idx)
    } else {
        Some(0)
    }
}

// Helper function to find next character boundary
fn find_next_char_boundary(s: &str, pos: usize) -> Option<usize> {
    if pos >= s.len() {
        return None;
    }

    // Find the next valid UTF-8 character boundary
    let mut idx = pos + 1;
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }

    if idx <= s.len() {
        Some(idx)
    } else {
        Some(s.len())
    }
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

    // Create an attractive success message box
    let message_lines = vec![
        "✓ Snippet added successfully!",
        "",
        "Your snippet is now ready to use.",
        "",
        "Press any key to view your snippets...",
    ];

    // Calculate box dimensions
    let box_width = 50u16;
    let box_height = (message_lines.len() + 4) as u16;
    let x = (width.saturating_sub(box_width)) / 2;
    let y = (height.saturating_sub(box_height)) / 2;

    // Multiple commands with individual error handling
    if let Err(e) = execute!(stdout, terminal::Clear(ClearType::All)) {
        return Err(SniptError::Other(format!("Failed to clear screen: {}", e)));
    }

    // Draw the success box
    if let Err(e) = execute!(
        stdout,
        // Draw top border
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Green),
        Print("╭"),
        Print("─".repeat((box_width - 2) as usize)),
        Print("╮"),
        // Draw title
        cursor::MoveTo(x + (box_width - 16) / 2, y),
        Print("╡ Success ╞"),
        // Reset for content
        ResetColor
    ) {
        return Err(SniptError::Other(format!("Failed to draw box top: {}", e)));
    }

    // Draw message content
    for (i, line) in message_lines.iter().enumerate() {
        let line_y = y + i as u16 + 2; // +2 to account for top border and spacing

        // Calculate position for centered text
        let text_x = if line.is_empty() {
            x + 2
        } else {
            x + (box_width - line.len() as u16) / 2
        };

        let color = if i == 0 {
            // Make the first line (success message) brighter
            Color::Green
        } else {
            Color::White
        };

        if let Err(e) = execute!(
            stdout,
            // Draw border
            cursor::MoveTo(x, line_y),
            SetForegroundColor(Color::Green),
            Print("│"),
            // Draw content
            cursor::MoveTo(text_x, line_y),
            SetForegroundColor(color),
            Print(line),
            // Draw border
            cursor::MoveTo(x + box_width - 1, line_y),
            SetForegroundColor(Color::Green),
            Print("│"),
            ResetColor
        ) {
            return Err(SniptError::Other(format!(
                "Failed to draw line {}: {}",
                i, e
            )));
        }
    }

    // Draw bottom border
    if let Err(e) = execute!(
        stdout,
        cursor::MoveTo(x, y + box_height - 1),
        SetForegroundColor(Color::Green),
        Print("╰"),
        Print("─".repeat((box_width - 2) as usize)),
        Print("╯"),
        ResetColor
    ) {
        return Err(SniptError::Other(format!(
            "Failed to draw box bottom: {}",
            e
        )));
    }

    if let Err(e) = stdout.flush() {
        return Err(SniptError::Other(format!("Failed to flush output: {}", e)));
    }

    // Wait for keypress but use cross-platform compatible wait instead of fixed sleep
    let exit_at = std::time::Instant::now() + Duration::from_millis(1000);
    while std::time::Instant::now() < exit_at {
        if crossterm::event::poll(Duration::from_millis(100))? {
            let _ = crossterm::event::read()?;
            break;
        }
    }

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
        Print("⚠ "),
        Print(display_msg),
        ResetColor
    ) {
        return Err(SniptError::Other(format!(
            "Failed to show error message: {}",
            e
        )));
    }

    if let Err(e) = stdout.flush() {
        return Err(SniptError::Other(format!("Failed to flush output: {}", e)));
    }

    Ok(())
}

fn thread_sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
