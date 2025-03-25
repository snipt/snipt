use crate::error::{Result, ScribeError};
use crate::models::SnippetEntry;
use crate::storage::load_snippets;

use arboard::Clipboard;
use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io::{self, stdout};

/// Display the snippet manager UI
pub fn display_snippet_manager() -> Result<()> {
    let entries = load_snippets().map_err(|e| {
        eprintln!("Failed to load snippets: {}", e);
        e
    })?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let result = run_ui(&mut terminal, &entries);

    // Clean up terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

/// Main UI loop
fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    entries: &[SnippetEntry],
) -> Result<()> {
    if entries.is_empty() {
        return show_empty_ui(terminal);
    }

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => return Err(ScribeError::Clipboard(e.to_string())),
    };

    let mut selected = entries.len().saturating_sub(1); // Start at the bottom
    let mut offset = 0; // Offset for scrolling

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let app_height = size.height.saturating_sub(4); // Leave room for help/instructions
            let max_visible_items = app_height.saturating_sub(2) as usize; // Account for borders

            // Ensure offset keeps the selected item in view
            if selected >= offset + max_visible_items {
                offset = selected.saturating_sub(max_visible_items).saturating_add(1);
            } else if selected < offset {
                offset = selected;
            }

            // Calculate visible entries
            let visible_entries = &entries[offset..entries.len().min(offset + max_visible_items)];

            // Render list items
            let items: Vec<ListItem> = visible_entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let actual_index = offset + i;
                    let elapsed = format!("{:>7}", entry.formatted_time());
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
                    .title(" Scribe Snippets ")
                    .style(Style::default().bg(Color::Black).fg(Color::White)),
            );

            let help_text = Paragraph::new(Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(": Navigate  "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(": Copy to clipboard  "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(": Exit"),
            ]))
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().bg(Color::Black));

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(app_height), Constraint::Length(3)].as_ref())
                .split(size);

            f.render_widget(list, layout[0]);
            f.render_widget(help_text, layout[1]);
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
                        selected = selected.saturating_add(1);
                    }
                }
                KeyCode::Enter => {
                    let content = &entries[selected].snippet;
                    if let Err(e) = clipboard.set_text(content.to_owned()) {
                        return Err(ScribeError::Clipboard(e.to_string()));
                    }
                    return Ok(());
                }
                KeyCode::Esc => return Ok(()),
                _ => {}
            }
        }
    }
}

fn show_empty_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal.draw(|f| {
        let size = f.size();

        let message = Paragraph::new(Line::from(vec![
            Span::raw("No snippets found. Add one with: "),
            Span::styled(
                "scribe add --shortcut <name> --snippet <text>",
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Scribe ")
                .style(Style::default().bg(Color::Black).fg(Color::White)),
        );

        let help = Paragraph::new("Press any key to exit").style(Style::default().fg(Color::Gray));

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(40),
                    Constraint::Min(3),
                    Constraint::Percentage(40),
                ]
                .as_ref(),
            )
            .split(size);

        f.render_widget(message, layout[1]);
        f.render_widget(
            help,
            Rect {
                x: 0,
                y: layout[1].bottom() + 1,
                width: size.width,
                height: 1,
            },
        );
    })?;

    // Wait for any key press
    event::read()?;
    Ok(())
}
