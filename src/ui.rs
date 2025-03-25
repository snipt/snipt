use crate::error::{Result, ScribeError};
use crate::models::SnippetEntry;
use crate::storage::{delete_snippet, load_snippets};

use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout};

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Filtering,
}

struct AppState {
    entries: Vec<SnippetEntry>,
    selected: usize,
    offset: usize,
    search_query: String,
    filtered_indices: Vec<usize>,
    input_mode: InputMode,
    tab_index: usize,
}

impl AppState {
    fn new(entries: Vec<SnippetEntry>) -> Self {
        let filtered_indices = (0..entries.len()).collect();
        Self {
            selected: entries.len().saturating_sub(1),
            entries,
            offset: 0,
            search_query: String::new(),
            filtered_indices,
            input_mode: InputMode::Normal,
            tab_index: 0,
        }
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.shortcut.to_lowercase().contains(&query)
                        || entry.snippet.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Adjust selected index if it's now out of bounds
        if !self.filtered_indices.is_empty() {
            if self.selected >= self.filtered_indices.len() {
                self.selected = self.filtered_indices.len() - 1;
            }
        } else {
            self.selected = 0;
        }
    }

    fn get_filtered_entry(&self, index: usize) -> Option<&SnippetEntry> {
        if index < self.filtered_indices.len() {
            let actual_index = self.filtered_indices[index];
            Some(&self.entries[actual_index])
        } else {
            None
        }
    }

    fn get_selected_entry_index(&self) -> Option<usize> {
        if self.selected < self.filtered_indices.len() {
            Some(self.filtered_indices[self.selected])
        } else {
            None
        }
    }

    fn get_current_tab(&self) -> &str {
        match self.tab_index {
            0 => "Snippets",
            1 => "Help",
            _ => "Snippets",
        }
    }
}

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
    let mut app_state = AppState::new(entries);
    app_state.apply_filter();

    let result = run_ui(&mut terminal, &mut app_state);

    // Clean up terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    if state.entries.is_empty() {
        return show_empty_ui(terminal);
    }

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => return Err(ScribeError::Clipboard(e.to_string())),
    };

    let mut should_refresh = false;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Create main layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Tabs area
                    Constraint::Min(5),    // Content area
                    Constraint::Length(1), // Filter input area
                    Constraint::Length(2), // Help text
                ])
                .split(size);

            // Render tab bar
            let titles = vec!["Snippets", "Help"]
                .iter()
                .map(|t| Span::styled(*t, Style::default().fg(Color::White)))
                .collect();

            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title(" Scribe "))
                .select(state.tab_index)
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(tabs, main_chunks[0]);

            match state.tab_index {
                0 => {
                    // Content layout - split into list and details
                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .split(main_chunks[1]);

                    render_snippet_list(f, state, content_chunks[0]);
                    render_snippet_details(f, state, content_chunks[1]);
                }
                1 => {
                    render_help_screen(f, main_chunks[1]);
                }
                _ => {}
            }

            // Render filter input
            let filter_style = match state.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Filtering => Style::default().fg(Color::Yellow),
            };

            let filter_text = if state.input_mode == InputMode::Filtering {
                format!("🔍 {}", state.search_query)
            } else {
                String::from("Press '/' to search")
            };

            let filter = Paragraph::new(filter_text)
                .style(filter_style)
                .alignment(Alignment::Left);
            f.render_widget(filter, main_chunks[2]);

            // Render status bar with keyboard shortcuts
            let status = render_status_bar(state);
            f.render_widget(status, main_chunks[3]);
        })?;

        if should_refresh {
            should_refresh = false;
            state.apply_filter();
        }

        // Handle input
        if let Ok(Event::Key(key)) = event::read() {
            match state.input_mode {
                InputMode::Normal => match key {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        return Ok(());
                    }
                    KeyEvent {
                        code: KeyCode::Char('1'),
                        ..
                    } => {
                        state.tab_index = 0;
                    }
                    KeyEvent {
                        code: KeyCode::Char('2'),
                        ..
                    } => {
                        state.tab_index = 1;
                    }
                    KeyEvent {
                        code: KeyCode::Tab, ..
                    } => {
                        state.tab_index = (state.tab_index + 1) % 2;
                    }
                    KeyEvent {
                        code: KeyCode::Char('/'),
                        ..
                    } => {
                        state.input_mode = InputMode::Filtering;
                    }
                    _ => {
                        if state.tab_index == 0 {
                            handle_list_input(state, &mut clipboard, key, &mut should_refresh)?;
                        }
                    }
                },
                InputMode::Filtering => match key {
                    KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        state.input_mode = InputMode::Normal;
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        state.input_mode = InputMode::Normal;
                        should_refresh = true;
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => {
                        state.search_query.push(c);
                        should_refresh = true;
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        state.search_query.pop();
                        should_refresh = true;
                    }
                    _ => {}
                },
            }
        }
    }
}

fn handle_list_input(
    state: &mut AppState,
    clipboard: &mut Clipboard,
    key: KeyEvent,
    should_refresh: &mut bool,
) -> Result<()> {
    match key {
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            if state.selected > 0 {
                state.selected = state.selected.saturating_sub(1);
            }
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            if !state.filtered_indices.is_empty()
                && state.selected < state.filtered_indices.len() - 1
            {
                state.selected += 1;
            }
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if let Some(actual_index) = state.get_selected_entry_index() {
                let content = &state.entries[actual_index].snippet;
                if let Err(e) = clipboard.set_text(content.to_owned()) {
                    return Err(ScribeError::Clipboard(e.to_string()));
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            if let Some(actual_index) = state.get_selected_entry_index() {
                let shortcut = state.entries[actual_index].shortcut.clone();
                if let Err(e) = delete_snippet(&shortcut) {
                    return Err(e);
                }

                // Reload entries
                match load_snippets() {
                    Ok(entries) => {
                        state.entries = entries;
                        *should_refresh = true;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_snippet_list<B: ratatui::backend::Backend>(
    f: &mut Frame<B>,
    state: &AppState,
    area: Rect,
) {
    let app_height = area.height.saturating_sub(2); // Account for borders
    let max_visible_items = app_height as usize;

    // Calculate optimal offset
    let offset = if !state.filtered_indices.is_empty() {
        if state.selected >= state.offset + max_visible_items {
            state
                .selected
                .saturating_sub(max_visible_items)
                .saturating_add(1)
        } else if state.selected < state.offset {
            state.selected
        } else {
            state.offset
        }
    } else {
        0
    };

    // Calculate visible entries
    let end_idx = (offset + max_visible_items).min(state.filtered_indices.len());
    let visible_range = offset..end_idx;

    // Render list items
    let items: Vec<ListItem> = visible_range
        .map(|i| {
            let entry = state.get_filtered_entry(i).unwrap();
            let shortcut_styled = Span::styled(
                format!("{:15}", entry.shortcut),
                Style::default().fg(Color::Cyan),
            );

            let snippet_preview = if entry.snippet.len() > 20 {
                format!("{}...", &entry.snippet[..17])
            } else {
                entry.snippet.clone()
            };

            let snippet_styled = Span::styled(snippet_preview, Style::default().fg(Color::White));

            let line = Line::from(vec![shortcut_styled, Span::raw(" "), snippet_styled]);

            ListItem::new(line)
        })
        .collect();

    let total_count = state.filtered_indices.len();
    let title = if state.search_query.is_empty() {
        format!(" Snippets ({}) ", total_count)
    } else {
        format!(
            " Filtered Snippets ({}/{}) ",
            total_count,
            state.entries.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    f.render_stateful_widget(
        list,
        area,
        &mut ratatui::widgets::ListState::default()
            .with_selected(Some(state.selected.saturating_sub(offset))),
    );
}

fn render_snippet_details<B: ratatui::backend::Backend>(
    f: &mut Frame<B>,
    state: &AppState,
    area: Rect,
) {
    let selected_entry = if !state.filtered_indices.is_empty() {
        state.get_filtered_entry(state.selected)
    } else {
        None
    };

    let block = Block::default().borders(Borders::ALL).title(" Details ");

    if let Some(entry) = selected_entry {
        let shortcut_line = Line::from(vec![
            Span::styled("Shortcut: ", Style::default().fg(Color::Yellow)),
            Span::styled(&entry.shortcut, Style::default().fg(Color::White)),
        ]);

        let timestamp_line = Line::from(vec![
            Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
            Span::styled(entry.formatted_time(), Style::default().fg(Color::Green)),
        ]);

        let snippet_label = Span::styled("Snippet:", Style::default().fg(Color::Yellow));
        let snippet_text = Span::styled(&entry.snippet, Style::default().fg(Color::White));

        let content = Text::from(vec![
            shortcut_line,
            timestamp_line,
            Line::from(""),
            Line::from(snippet_label),
            Line::from(snippet_text),
        ]);

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No snippet selected")
            .block(block)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}

fn render_help_screen<B: ratatui::backend::Backend>(f: &mut Frame<B>, area: Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Scribe ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("is a text snippet expansion tool that lets you expand shortcuts as you type."),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(Color::Green)),
            Span::raw(": Navigate through snippets"),
        ]),
        Line::from(vec![
            Span::styled("  Tab", Style::default().fg(Color::Green)),
            Span::raw(": Switch between tabs"),
        ]),
        Line::from(vec![
            Span::styled("  1/2", Style::default().fg(Color::Green)),
            Span::raw(": Quick tab selection"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw(": Copy snippet to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Green)),
            Span::raw(": Search snippets"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+D", Style::default().fg(Color::Green)),
            Span::raw(": Delete current snippet"),
        ]),
        Line::from(vec![
            Span::styled("  Esc/q", Style::default().fg(Color::Green)),
            Span::raw(": Exit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Usage Tips", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("• Scribe expands text starting with the special character ':' followed by your shortcut."),
        Line::from("• Add new snippets with: scribe add --shortcut <name> --snippet <text>"),
        Line::from("• Start the daemon with: scribe start"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help & Usage "),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_status_bar(state: &AppState) -> Paragraph<'static> {
    let help_text = match state.get_current_tab() {
        "Snippets" => match state.input_mode {
            InputMode::Normal => {
                "↑↓:Navigate | Enter:Copy | /:Search | Ctrl+D:Delete | Tab:Switch Tab | Esc/q:Exit"
            }
            InputMode::Filtering => "Enter:Apply Filter | Esc:Cancel",
        },
        "Help" => "Tab:Switch | Esc/q:Exit",
        _ => "",
    };

    Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
}

fn show_empty_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal.draw(|f| {
        let size = f.size();

        let message = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "No snippets found",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Add your first snippet with one of these commands:"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "scribe add --shortcut <name> --snippet <text>",
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("scribe new", Style::default().fg(Color::Cyan)),
                Span::raw(" (interactive mode)"),
            ]),
            Line::from(""),
            Line::from("After adding snippets, start the daemon with:"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("scribe start", Style::default().fg(Color::Cyan)),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Welcome to Scribe ")
                .style(Style::default().bg(Color::Black).fg(Color::White)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let help = Paragraph::new("Press any key to exit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(30),
                    Constraint::Min(12),
                    Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(size);

        f.render_widget(message, layout[1]);
        f.render_widget(
            help,
            Rect {
                x: 0,
                y: layout[1].bottom() + 2,
                width: size.width,
                height: 1,
            },
        );
    })?;

    // Wait for any key press
    event::read()?;
    Ok(())
}
