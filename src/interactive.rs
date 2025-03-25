use crate::error::Result;
use crate::storage::add_snippet;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, stdout, Write};

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
    let mut snippet = String::new();
    let mut current_field = 0; // 0 for shortcut, 1 for snippet
    let mut cursor_pos = 0;

    loop {
        // Clear screen and redraw UI
        draw_ui(stdout, &shortcut, &snippet, current_field, cursor_pos)?;

        // Handle input
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match code {
                KeyCode::Tab | KeyCode::Down => {
                    // Switch between fields
                    current_field = (current_field + 1) % 2;
                    cursor_pos = if current_field == 0 {
                        shortcut.len()
                    } else {
                        snippet.len()
                    };
                }
                KeyCode::BackTab | KeyCode::Up => {
                    // Switch between fields (reverse)
                    current_field = (current_field + 1) % 2;
                    cursor_pos = if current_field == 0 {
                        shortcut.len()
                    } else {
                        snippet.len()
                    };
                }
                KeyCode::Enter => {
                    if current_field == 0 {
                        // Move to snippet field when pressing Enter in shortcut field
                        current_field = 1;
                        cursor_pos = snippet.len();
                    } else {
                        // Submit when pressing Enter in snippet field
                        if !shortcut.is_empty() && !snippet.is_empty() {
                            add_snippet(shortcut, snippet)?;
                            show_success_message(stdout)?;
                            return Ok(());
                        } else {
                            show_error_message(stdout, "Both fields must be filled")?;
                            thread_sleep(1500);
                        }
                    }
                }
                KeyCode::Esc => {
                    return Ok(());
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        if current_field == 0 {
                            shortcut.remove(cursor_pos - 1);
                        } else {
                            snippet.remove(cursor_pos - 1);
                        }
                        cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let current_str = if current_field == 0 {
                        &mut shortcut
                    } else {
                        &mut snippet
                    };
                    if cursor_pos < current_str.len() {
                        current_str.remove(cursor_pos);
                    }
                }
                KeyCode::Left => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let max_pos = if current_field == 0 {
                        shortcut.len()
                    } else {
                        snippet.len()
                    };
                    if cursor_pos < max_pos {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    cursor_pos = 0;
                }
                KeyCode::End => {
                    cursor_pos = if current_field == 0 {
                        shortcut.len()
                    } else {
                        snippet.len()
                    };
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char(c) => {
                    if current_field == 0 {
                        shortcut.insert(cursor_pos, c);
                    } else {
                        snippet.insert(cursor_pos, c);
                    }
                    cursor_pos += 1;
                }
                _ => {}
            }
        }
    }
}

fn draw_ui(
    stdout: &mut io::Stdout,
    shortcut: &str,
    snippet: &str,
    current_field: usize,
    cursor_pos: usize,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let panel_width = width.saturating_sub(10);
    let panel_height = 12;
    let start_x = 5;
    let start_y = (height - panel_height) / 2;

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

    // Draw snippet field
    draw_field(
        stdout,
        field_x,
        start_y + 6,
        field_width,
        "Snippet:",
        snippet,
        current_field == 1,
    )?;

    // Draw help text
    let help_text = "Tab: Switch fields | Enter: Submit | Esc: Cancel";
    let help_x = start_x + (panel_width - help_text.len() as u16) / 2;
    execute!(
        stdout,
        cursor::MoveTo(help_x, start_y + panel_height - 2),
        SetForegroundColor(Color::DarkGrey),
        Print(help_text),
        ResetColor
    )?;

    let field_y = if current_field == 0 {
        start_y + 3
    } else {
        start_y + 7
    };
    let visible_cursor_pos = cursor_pos.min(field_width as usize - 3);

    execute!(
        stdout,
        cursor::MoveTo(field_x + 1 + visible_cursor_pos as u16, field_y),
        cursor::Show
    )?;

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
    let panel_height = 12;
    let start_y = (height - panel_height) / 2;
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
