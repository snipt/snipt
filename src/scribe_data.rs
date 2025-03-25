use arboard::Clipboard;
use chrono::{DateTime, Local, TimeZone};
use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self};
use std::io::{self, stdout};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct ScribeData {
    pub shortcut: String,
    pub snippet: String,
    pub timestamp: String,
}

pub fn add_snippet(shortcut: String, snippet: String) {
    // Try to get home directory, fallback to current directory
    // Get the home directory from the HOME env var (works on Linux/macOS)
    let data_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| {
            println!("Warning: HOME not set. Using current directory.");
            PathBuf::from(".scribe")
        });

    // Create the directory if it doesn't exist
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).expect("Failed to create .scribe directory");
    }
    let scribe_config = data_dir.join("scribe.json");
    let timestamp = Local::now().to_rfc3339();

    let entry = ScribeData {
        shortcut,
        snippet,
        timestamp,
    };

    // Load existing data
    let mut data = if let Ok(content) = fs::read_to_string(&scribe_config) {
        serde_json::from_str::<Vec<ScribeData>>(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };

    data.push(entry);

    // Serialize and write updated history
    let serialized_history =
        serde_json::to_string_pretty(&data).expect("Failed to serialize clipboard history");
    fs::write(&scribe_config, serialized_history).expect("Failed to write clipboard history");
}

pub fn delete_snippet(shortcut: String) {
    let data_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let scribe_config = data_dir.join("scribe.json");

    let mut data = if let Ok(content) = fs::read_to_string(&scribe_config) {
        serde_json::from_str::<Vec<ScribeData>>(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };

    data.retain(|entry| entry.shortcut != shortcut);

    let serialized_history =
        serde_json::to_string_pretty(&data).expect("Failed to serialize clipboard history");
    fs::write(&scribe_config, serialized_history).expect("Failed to write clipboard history");
}

pub fn update_snippet(shortcut: String, snippet: String) {
    let data_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| PathBuf::from(".scribe"));

    let scribe_config = data_dir.join("scribe.json");

    let mut data = if let Ok(content) = fs::read_to_string(&scribe_config) {
        serde_json::from_str::<Vec<ScribeData>>(&content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };

    for entry in &mut data {
        if entry.shortcut == shortcut {
            entry.snippet = snippet.clone();
            entry.timestamp = Local::now().to_rfc3339();
        }
    }

    let serialized_history =
        serde_json::to_string_pretty(&data).expect("Failed to serialize clipboard history");
    fs::write(&scribe_config, serialized_history).expect("Failed to write clipboard history");
}

pub fn load_scribe() -> Result<Vec<ScribeData>, io::Error> {
    let data_dir = env::var("HOME")
        .map(|home| PathBuf::from(home).join(".scribe"))
        .unwrap_or_else(|_| {
            println!("Warning: HOME not set. Using current directory.");
            PathBuf::from(".scribe")
        });

    let scribe_config = data_dir.join("scribe.json");

    // Ensure the scribe directory exists
    if fs::metadata(data_dir).is_err() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Scribe directory not found",
        ));
    }

    // Read the contents of the config file
    let content = fs::read_to_string(&scribe_config)?;

    // Deserialize the JSON content into a vector of ScribeData
    if content.trim() == "[]" {
        return Ok(vec![]);
    }
    // Ensure the JSON is valid and not empty
    let trimmed_content = content.trim();
    if trimmed_content.is_empty() || !trimmed_content.starts_with('[') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid or empty scribe config file",
        ));
    }

    Ok(serde_json::from_str(trimmed_content)?)
}

pub fn print_scribe() -> Result<(), io::Error> {
    let entries = load_scribe().map_err(|e| {
        eprintln!("Failed to load scribe config: {}", e);
        e
    })?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let result = run_app(&mut terminal, &entries);

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    result
}

fn format_elapsed_time(timestamp: &str) -> String {
    let entry_time = DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Local))
        .unwrap_or_else(|_| Local.timestamp_opt(0, 0).unwrap());
    let now = Local::now();
    let duration = now.signed_duration_since(entry_time);

    let formatted = if duration.num_seconds() < 60 {
        format!("{}s ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    };

    format!("{:>7}", formatted) // Right-align with 7 characters
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    entries: &[ScribeData],
) -> io::Result<()> {
    let mut clipboard = Clipboard::new().unwrap();
    let mut selected = entries.len().saturating_sub(1); // Start at the bottom
    let mut offset = 0; // Offset to manage scrolling

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let app_height = size.height / 2;
            let max_visible_items = app_height.saturating_sub(2) as usize; // Account for borders

            // Ensure offset keeps the selected item in view
            if selected >= offset + max_visible_items {
                offset = selected.saturating_sub(max_visible_items).saturating_add(1);
            } else if selected < offset {
                offset = selected;
            }

            // Calculate the visible entries
            let visible_entries = &entries[offset..entries.len().min(offset + max_visible_items)];

            // Render list items
            let items: Vec<ListItem> = visible_entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let actual_index = offset + i;
                    let elapsed = format_elapsed_time(&entry.timestamp);
                    let elapsed_styled = Span::styled(elapsed, Style::default().fg(Color::Green));
                    let snippet_styled =
                        Span::styled(entry.snippet.clone(), Style::default().fg(Color::White));
                    let shortcut_styled =
                        Span::styled(entry.shortcut.clone(), Style::default().fg(Color::Red));

                    let highlight_symbol = if actual_index == selected {
                        Span::styled(
                            "> ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("  ")
                    };

                    let line = Line::from(vec![
                        highlight_symbol,
                        elapsed_styled,
                        Span::raw(" "),
                        shortcut_styled,
                        Span::raw(" "),
                        snippet_styled,
                    ]);

                    if actual_index == selected {
                        ListItem::new(line).style(Style::default().bg(Color::DarkGray))
                    } else {
                        ListItem::new(line)
                    }
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" scribe ")
                    .style(Style::default().bg(Color::Black).fg(Color::White)),
            );

            f.render_widget(
                list,
                Rect {
                    y: size.height - app_height,
                    width: size.width,
                    height: app_height,
                    ..size
                },
            );
        })?;

        // Handle input
        if let event::Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected = selected.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if selected < entries.len().saturating_sub(1) {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    let content = &entries[selected].snippet;
                    clipboard.set_text(content.to_owned()).unwrap();
                    println!("Copied: {}", content);
                    break;
                }
                KeyCode::Esc => break,
                _ => {}
            }
        }
    }
    Ok(())
}
