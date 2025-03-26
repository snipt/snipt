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

#[derive(PartialEq, Copy, Clone)]
enum EditorMode {
    Normal, // For navigation and commands
    Insert, // For text insertion
}

pub fn interactive_add() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Run the interactive UI
    let result = run_interactive_ui(&mut stdout);

    // Cleanup terminal
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_interactive_ui(stdout: &mut io::Stdout) -> Result<()> {
    let mut shortcut = String::new();
    let mut snippet = Vec::new(); // Changed to Vec<String> for multiline support
    snippet.push(String::new()); // Start with one empty line
    let mut current_field = 0; // 0 for shortcut, 1 for snippet
    let mut cursor_pos = 0;
    let mut current_line = 0; // Track current line in multiline editing

    let mut editor_mode = EditorMode::Insert; // Start in insert mode for convenience

    loop {
        // Clear screen and redraw UI
        draw_ui(
            stdout,
            &shortcut,
            &snippet,
            current_field,
            cursor_pos,
            current_line,
            editor_mode,
        )?;

        // Handle input
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match code {
                KeyCode::Tab | KeyCode::Down => {
                    if current_field == 0 {
                        // Switch from shortcut to snippet
                        current_field = 1;
                        current_line = 0;
                        cursor_pos = snippet[current_line].len();
                    } else if modifiers.contains(KeyModifiers::SHIFT) {
                        // Switch back to shortcut with Shift+Tab
                        current_field = 0;
                        cursor_pos = shortcut.len();
                    } else if current_line < snippet.len() - 1 {
                        // Move down in multiline snippet
                        current_line += 1;
                        cursor_pos = snippet[current_line].len().min(cursor_pos);
                    }
                }
                KeyCode::BackTab | KeyCode::Up => {
                    if current_field == 1 {
                        if current_line > 0 {
                            // Move up in multiline snippet
                            current_line -= 1;
                            cursor_pos = snippet[current_line].len().min(cursor_pos);
                        } else {
                            // Switch to shortcut field
                            current_field = 0;
                            cursor_pos = shortcut.len();
                        }
                    }
                }
                KeyCode::Esc => {
                    return Ok(());
                }
                _ => {
                    if current_field == 0 {
                        // Shortcut field handling
                        match code {
                            KeyCode::Enter => {
                                // Move to snippet field
                                current_field = 1;
                                current_line = 0;
                                cursor_pos = snippet[current_line].len();
                            }
                            KeyCode::Backspace => {
                                if cursor_pos > 0 {
                                    shortcut.remove(cursor_pos - 1);
                                    cursor_pos -= 1;
                                }
                            }
                            KeyCode::Delete => {
                                if cursor_pos < shortcut.len() {
                                    shortcut.remove(cursor_pos);
                                }
                            }
                            KeyCode::Left => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if cursor_pos < shortcut.len() {
                                    cursor_pos += 1;
                                }
                            }
                            KeyCode::Home => {
                                cursor_pos = 0;
                            }
                            KeyCode::End => {
                                cursor_pos = shortcut.len();
                            }
                            KeyCode::Char(c) => {
                                shortcut.insert(cursor_pos, c);
                                cursor_pos += 1;
                            }
                            _ => {}
                        }
                    } else {
                        // Snippet field handling with vim-like modes
                        match editor_mode {
                            EditorMode::Normal => {
                                match code {
                                    KeyCode::Char('i') => {
                                        // Enter insert mode
                                        editor_mode = EditorMode::Insert;
                                    }
                                    KeyCode::Char('a') => {
                                        // Enter insert mode after cursor
                                        if cursor_pos < snippet[current_line].len() {
                                            cursor_pos += 1;
                                        }
                                        editor_mode = EditorMode::Insert;
                                    }
                                    KeyCode::Char('A') => {
                                        // Enter insert mode at end of line
                                        cursor_pos = snippet[current_line].len();
                                        editor_mode = EditorMode::Insert;
                                    }
                                    KeyCode::Char('o') => {
                                        // Open new line below and enter insert mode
                                        snippet.insert(current_line + 1, String::new());
                                        current_line += 1;
                                        cursor_pos = 0;
                                        editor_mode = EditorMode::Insert;
                                    }
                                    KeyCode::Char('O') => {
                                        // Open new line above and enter insert mode
                                        snippet.insert(current_line, String::new());
                                        cursor_pos = 0;
                                        editor_mode = EditorMode::Insert;
                                    }
                                    KeyCode::Char('h') => {
                                        // Move cursor left
                                        if cursor_pos > 0 {
                                            cursor_pos -= 1;
                                        } else if current_line > 0 {
                                            current_line -= 1;
                                            cursor_pos = snippet[current_line].len();
                                        }
                                    }
                                    KeyCode::Char('l') => {
                                        // Move cursor right
                                        if cursor_pos < snippet[current_line].len() {
                                            cursor_pos += 1;
                                        } else if current_line < snippet.len() - 1 {
                                            current_line += 1;
                                            cursor_pos = 0;
                                        }
                                    }
                                    KeyCode::Char('j') => {
                                        // Move cursor down
                                        if current_line < snippet.len() - 1 {
                                            current_line += 1;
                                            cursor_pos =
                                                cursor_pos.min(snippet[current_line].len());
                                        }
                                    }
                                    KeyCode::Char('k') => {
                                        // Move cursor up
                                        if current_line > 0 {
                                            current_line -= 1;
                                            cursor_pos =
                                                cursor_pos.min(snippet[current_line].len());
                                        }
                                    }
                                    KeyCode::Char('0') => {
                                        // Move to beginning of line
                                        cursor_pos = 0;
                                    }
                                    KeyCode::Char('$') => {
                                        // Move to end of line
                                        cursor_pos = snippet[current_line].len();
                                    }
                                    KeyCode::Char('d')
                                        if modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        // Delete current line (dd in vim)
                                        if snippet.len() > 1 {
                                            snippet.remove(current_line);
                                            if current_line >= snippet.len() {
                                                current_line = snippet.len() - 1;
                                            }
                                            cursor_pos =
                                                cursor_pos.min(snippet[current_line].len());
                                        } else {
                                            // Don't remove the last line, just clear it
                                            snippet[0].clear();
                                            cursor_pos = 0;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        // Complete editing and submit
                                        if !shortcut.is_empty()
                                            && !snippet.is_empty()
                                            && !snippet[0].is_empty()
                                        {
                                            // Join the lines with newlines
                                            let full_snippet = snippet.join("\n");
                                            match add_snippet(shortcut.clone(), full_snippet) {
                                                Ok(_) => {
                                                    show_success_message(stdout)?;
                                                    return Ok(());
                                                }
                                                Err(ScribeError::Other(msg))
                                                    if msg.contains("already exists") =>
                                                {
                                                    show_error_message(stdout, &msg)?;
                                                    thread_sleep(1500);
                                                }
                                                Err(e) => return Err(e),
                                            }
                                        } else {
                                            show_error_message(
                                                stdout,
                                                "Both fields must be filled",
                                            )?;
                                            thread_sleep(1500);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            EditorMode::Insert => {
                                match code {
                                    KeyCode::Esc => {
                                        // Return to normal mode
                                        editor_mode = EditorMode::Normal;
                                    }
                                    KeyCode::Enter => {
                                        // Insert a new line at cursor position
                                        let rest_of_line =
                                            if cursor_pos < snippet[current_line].len() {
                                                snippet[current_line][cursor_pos..].to_string()
                                            } else {
                                                String::new()
                                            };

                                        snippet[current_line].truncate(cursor_pos);
                                        snippet.insert(current_line + 1, rest_of_line);
                                        current_line += 1;
                                        cursor_pos = 0;
                                    }
                                    KeyCode::Backspace => {
                                        // Delete character before cursor
                                        if cursor_pos > 0 {
                                            snippet[current_line].remove(cursor_pos - 1);
                                            cursor_pos -= 1;
                                        } else if current_line > 0 {
                                            // At start of line, merge with previous line
                                            let current_content = snippet.remove(current_line);
                                            cursor_pos = snippet[current_line - 1].len();
                                            snippet[current_line - 1].push_str(&current_content);
                                            current_line -= 1;
                                        }
                                    }
                                    KeyCode::Delete => {
                                        // Delete character under cursor
                                        if cursor_pos < snippet[current_line].len() {
                                            snippet[current_line].remove(cursor_pos);
                                        } else if current_line < snippet.len() - 1 {
                                            // At end of line, merge with next line
                                            let next_content = snippet.remove(current_line + 1);
                                            snippet[current_line].push_str(&next_content);
                                        }
                                    }
                                    KeyCode::Left => {
                                        if cursor_pos > 0 {
                                            cursor_pos -= 1;
                                        } else if current_line > 0 {
                                            current_line -= 1;
                                            cursor_pos = snippet[current_line].len();
                                        }
                                    }
                                    KeyCode::Right => {
                                        if cursor_pos < snippet[current_line].len() {
                                            cursor_pos += 1;
                                        } else if current_line < snippet.len() - 1 {
                                            current_line += 1;
                                            cursor_pos = 0;
                                        }
                                    }
                                    KeyCode::Up => {
                                        if current_line > 0 {
                                            current_line -= 1;
                                            cursor_pos =
                                                cursor_pos.min(snippet[current_line].len());
                                        }
                                    }
                                    KeyCode::Down => {
                                        if current_line < snippet.len() - 1 {
                                            current_line += 1;
                                            cursor_pos =
                                                cursor_pos.min(snippet[current_line].len());
                                        }
                                    }
                                    KeyCode::Home => {
                                        cursor_pos = 0;
                                    }
                                    KeyCode::End => {
                                        cursor_pos = snippet[current_line].len();
                                    }
                                    KeyCode::Tab => {
                                        // Insert 4 spaces for indentation
                                        for _ in 0..4 {
                                            snippet[current_line].insert(cursor_pos, ' ');
                                            cursor_pos += 1;
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        snippet[current_line].insert(cursor_pos, c);
                                        cursor_pos += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
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
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let panel_width = width.saturating_sub(10);
    let panel_height = height.saturating_sub(8); // Larger panel for multiline content
    let start_x = 5;
    let start_y = 4;

    // Clear screen
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        ResetColor
    )?;

    // Draw title
    let title = " Add New Snippet ";
    let title_x = start_x + (panel_width - title.len() as u16) / 2;
    execute!(
        stdout,
        cursor::MoveTo(title_x, start_y - 1),
        SetForegroundColor(Color::Cyan),
        Print(title),
        ResetColor
    )?;

    // Draw panel
    draw_box(stdout, start_x, start_y, panel_width, panel_height)?;

    // Draw shortcut field
    let field_x = start_x + 3;
    let field_width = panel_width - 6;
    draw_field(
        stdout,
        field_x,
        start_y + 2,
        field_width,
        "Shortcut:",
        shortcut,
        current_field == 0,
    )?;

    // Draw snippet field (multiline)
    draw_multiline_field(
        stdout,
        field_x,
        start_y + 6,
        field_width,
        panel_height - 10, // Allow room for the multiline field
        "Snippet:",
        snippet,
        current_field == 1,
        current_line,
    )?;

    // Draw help text
    let normal_help =
        "i/a: Insert | o/O: New line | h/j/k/l: Navigate | Ctrl+d: Delete line | Enter: Submit";
    let insert_help = "Esc: Normal mode | Enter: New line | Tab: Indent | Arrows: Navigate";
    let help_text = if current_field == 1 {
        match editor_mode {
            EditorMode::Normal => normal_help,
            EditorMode::Insert => insert_help,
        }
    } else {
        "Tab: Next field | Enter: Submit | Esc: Cancel"
    };
    let help_x = start_x + (panel_width - help_text.len() as u16) / 2;
    execute!(
        stdout,
        cursor::MoveTo(help_x, start_y + panel_height - 2),
        SetForegroundColor(Color::DarkGrey),
        Print(help_text),
        ResetColor
    )?;

    if current_field == 1 {
        let mode_text = match editor_mode {
            EditorMode::Normal => "-- NORMAL --",
            EditorMode::Insert => "-- INSERT --",
        };

        execute!(
            stdout,
            cursor::MoveTo(field_x, start_y + 5),
            SetForegroundColor(if matches!(editor_mode, EditorMode::Normal) {
                Color::Blue
            } else {
                Color::Green
            }),
            Print(mode_text),
            ResetColor
        )?;
    }
    // Position cursor
    if current_field == 0 {
        let visible_cursor_pos = cursor_pos.min(field_width as usize - 3);
        execute!(
            stdout,
            cursor::MoveTo(field_x + 1 + visible_cursor_pos as u16, start_y + 3),
            cursor::Show
        )?;
    } else {
        // Position cursor in multiline field
        let visible_cursor_pos = cursor_pos.min(field_width as usize - 3);
        let line_offset = current_line.min(panel_height as usize - 12); // Limit visible lines
        execute!(
            stdout,
            cursor::MoveTo(
                field_x + 1 + visible_cursor_pos as u16,
                start_y + 7 + line_offset as u16
            ),
            cursor::Show
        )?;
    }

    stdout.flush()?;
    Ok(())
}

fn draw_box(stdout: &mut io::Stdout, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
    // Top border
    execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Blue),
        Print("╭"),
        Print("─".repeat((width - 2) as usize)),
        Print("╮")
    )?;

    // Side borders
    for i in 1..height - 1 {
        execute!(
            stdout,
            cursor::MoveTo(x, y + i),
            Print("│"),
            cursor::MoveTo(x + width - 1, y + i),
            Print("│")
        )?;
    }

    // Bottom border
    execute!(
        stdout,
        cursor::MoveTo(x, y + height - 1),
        Print("╰"),
        Print("─".repeat((width - 2) as usize)),
        Print("╯"),
        ResetColor
    )?;

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
    execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Yellow),
        Print(label),
        ResetColor
    )?;

    // Draw field box
    let visible_value = if value.len() > (width as usize - 4) {
        &value[value.len() - (width as usize - 4)..]
    } else {
        value
    };

    execute!(
        stdout,
        cursor::MoveTo(x, y + 1),
        SetForegroundColor(Color::Blue),
        Print("┌"),
        Print("─".repeat((width - 2) as usize)),
        Print("┐"),
        ResetColor
    )?;

    let bg_color = if active {
        Color::DarkBlue
    } else {
        Color::Black
    };
    let fg_color = if active { Color::White } else { Color::Grey };

    execute!(
        stdout,
        cursor::MoveTo(x, y + 2),
        SetForegroundColor(Color::Blue),
        Print("│"),
        SetBackgroundColor(bg_color),
        SetForegroundColor(fg_color),
        Print(" "),
        Print(visible_value),
        Print(" ".repeat((width - 3 - visible_value.len() as u16) as usize)),
        ResetColor,
        SetForegroundColor(Color::Blue),
        Print("│"),
        ResetColor
    )?;

    execute!(
        stdout,
        cursor::MoveTo(x, y + 3),
        SetForegroundColor(Color::Blue),
        Print("└"),
        Print("─".repeat((width - 2) as usize)),
        Print("┘"),
        ResetColor
    )?;

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
    execute!(
        stdout,
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Yellow),
        Print(label),
        ResetColor
    )?;

    // Draw field box
    execute!(
        stdout,
        cursor::MoveTo(x, y + 1),
        SetForegroundColor(Color::Blue),
        Print("┌"),
        Print("─".repeat((width - 2) as usize)),
        Print("┐"),
        ResetColor
    )?;

    let bg_color = if active { Color::Blue } else { Color::Black };
    let fg_color = if active { Color::White } else { Color::Grey };

    // Determine visible lines based on scroll position
    let visible_lines = if lines.len() as u16 > height {
        let start_line = current_line.saturating_sub((height / 2) as usize);
        start_line..(start_line + height as usize).min(lines.len())
    } else {
        0..lines.len()
    };

    // Draw each line
    for (i, line_idx) in visible_lines.clone().enumerate() {
        if i as u16 >= height {
            break;
        }

        let line = &lines[line_idx];
        let visible_line = if line.len() > (width as usize - 4) {
            &line[..width as usize - 7] // Leave room for ellipsis
        } else {
            line
        };

        let line_bg = if line_idx == current_line && active {
            bg_color
        } else {
            Color::Black
        };

        let padding = if line.len() > (width as usize - 4) {
            "..."
        } else {
            &" ".repeat((width - 3 - visible_line.len() as u16) as usize)
        };
        execute!(
            stdout,
            cursor::MoveTo(x, y + 2 + i as u16),
            SetForegroundColor(Color::Blue),
            Print("│"),
            SetBackgroundColor(line_bg),
            SetForegroundColor(fg_color),
            Print(" "),
            Print(visible_line),
            Print(padding),
            ResetColor,
            SetForegroundColor(Color::Blue),
            Print("│"),
            ResetColor
        )?;
    }

    // Fill remaining lines with empty space
    for i in visible_lines.len()..height as usize {
        execute!(
            stdout,
            cursor::MoveTo(x, y + 2 + i as u16),
            SetForegroundColor(Color::Blue),
            Print("│"),
            SetBackgroundColor(Color::Black),
            Print(" ".repeat((width - 2) as usize)),
            ResetColor,
            SetForegroundColor(Color::Blue),
            Print("│"),
            ResetColor
        )?;
    }

    execute!(
        stdout,
        cursor::MoveTo(x, y + 2 + height),
        SetForegroundColor(Color::Blue),
        Print("└"),
        Print("─".repeat((width - 2) as usize)),
        Print("┘"),
        ResetColor
    )?;

    // Draw multiline help text
    execute!(
        stdout,
        cursor::MoveTo(x, y + 3 + height),
        SetForegroundColor(Color::DarkGrey),
        Print("Enter: New line | ↑/↓: Navigate lines | Ctrl+Enter: Submit"),
        ResetColor
    )?;

    Ok(())
}

fn show_success_message(stdout: &mut io::Stdout) -> Result<()> {
    let (width, height) = terminal::size()?;
    let message = "✓ Snippet added successfully!";
    let x = (width - message.len() as u16) / 2;
    let y = height / 2;

    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(x, y),
        SetForegroundColor(Color::Green),
        Print(message),
        ResetColor
    )?;

    stdout.flush()?;
    thread_sleep(1500);
    Ok(())
}

fn show_error_message(stdout: &mut io::Stdout, message: &str) -> Result<()> {
    let (width, _) = terminal::size()?;
    let x = (width - message.len() as u16) / 2;

    // Find the bottom of our UI panel
    let (_, height) = terminal::size()?;
    let panel_height = height.saturating_sub(8);
    let start_y = 4;
    let error_y = start_y + panel_height;

    execute!(
        stdout,
        cursor::MoveTo(x, error_y),
        SetForegroundColor(Color::Red),
        Print(message),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

fn thread_sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
