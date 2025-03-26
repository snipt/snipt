use crate::error::{Result, ScribeError};
use crate::models::SnippetEntry;
use crate::storage::{delete_snippet, load_snippets, update_snippet};

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
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout};

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Filtering,
    Editing,    // Mode for editing a snippet
    Confirming, // Mode for confirming actions (delete)
}

enum ConfirmAction {
    Delete,
    // Can add more confirmation actions later
}

struct AppState {
    entries: Vec<SnippetEntry>,
    selected: usize,
    offset: usize,
    search_query: String,
    filtered_indices: Vec<usize>,
    input_mode: InputMode,
    tab_index: usize,
    edit_buffer: String,                   // Buffer for editing snippets
    confirm_action: Option<ConfirmAction>, // Track what we're confirming
}

impl AppState {
    fn new(entries: Vec<SnippetEntry>) -> Self {
        let filtered_indices = (0..entries.len()).collect();
        let selected = if entries.is_empty() {
            0
        } else {
            entries.len() - 1
        };

        Self {
            selected,
            entries,
            offset: 0,
            search_query: String::new(),
            filtered_indices,
            input_mode: InputMode::Normal,
            tab_index: 0,
            edit_buffer: String::new(),
            confirm_action: None,
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

        // Adjust selected index based on filtered results
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    fn get_filtered_entry(&self, index: usize) -> Option<&SnippetEntry> {
        if self.filtered_indices.is_empty() || index >= self.filtered_indices.len() {
            return None;
        }

        let actual_index = self.filtered_indices[index];
        Some(&self.entries[actual_index])
    }

    fn get_selected_entry_index(&self) -> Option<usize> {
        if self.filtered_indices.is_empty() || self.selected >= self.filtered_indices.len() {
            return None;
        }

        Some(self.filtered_indices[self.selected])
    }

    fn get_current_tab(&self) -> &str {
        match self.tab_index {
            0 => "Snippets",
            1 => "Help",
            _ => "Snippets",
        }
    }

    fn start_editing(&mut self) {
        if let Some(actual_index) = self.get_selected_entry_index() {
            self.edit_buffer = self.entries[actual_index].snippet.clone();
            self.input_mode = InputMode::Editing;
        }
    }

    fn save_edited_snippet(&mut self) -> Result<()> {
        if let Some(actual_index) = self.get_selected_entry_index() {
            let shortcut = self.entries[actual_index].shortcut.clone();
            let new_snippet = self.edit_buffer.clone();

            // Update in-memory entry
            self.entries[actual_index].update_snippet(new_snippet.clone());

            // Save to storage
            update_snippet(&shortcut, new_snippet)?;
        }
        self.input_mode = InputMode::Normal;
        Ok(())
    }

    fn start_delete_confirmation(&mut self) {
        if self.get_selected_entry_index().is_some() {
            self.confirm_action = Some(ConfirmAction::Delete);
            self.input_mode = InputMode::Confirming;
        }
    }

    // New: Update entries safely after deletion or any other operation
    fn update_entries(&mut self, new_entries: Vec<SnippetEntry>) {
        self.entries = new_entries;

        // Regenerate filtered indices
        self.apply_filter();

        // Update selection
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
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
                    Constraint::Length(1), // Filter/edit input area
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

            // Render filter/edit input area based on current mode
            match state.input_mode {
                InputMode::Normal => {
                    let filter = Paragraph::new("Press '/' to search")
                        .style(Style::default())
                        .alignment(Alignment::Left);
                    f.render_widget(filter, main_chunks[2]);
                }
                InputMode::Filtering => {
                    let filter = Paragraph::new(format!("🔍 {}", state.search_query))
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(Alignment::Left);
                    f.render_widget(filter, main_chunks[2]);
                }
                InputMode::Editing => {
                    let edit_text = format!("Edit: {}", state.edit_buffer);
                    let edit = Paragraph::new(edit_text)
                        .style(Style::default().fg(Color::Green))
                        .alignment(Alignment::Left);
                    f.render_widget(edit, main_chunks[2]);
                }
                InputMode::Confirming => {
                    // Don't change the filter area during confirmation
                }
            }

            // Render confirmation dialog if needed
            if state.input_mode == InputMode::Confirming {
                render_confirmation_dialog(f, state, size);
            }

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
                    KeyEvent {
                        code: KeyCode::Char('e'),
                        ..
                    } => {
                        if state.tab_index == 0 && !state.filtered_indices.is_empty() {
                            state.start_editing();
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('d'),
                        ..
                    } => {
                        if state.tab_index == 0 && !state.filtered_indices.is_empty() {
                            state.start_delete_confirmation();
                        }
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
                InputMode::Editing => match key {
                    KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        state.input_mode = InputMode::Normal;
                        state.edit_buffer.clear();
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        if let Err(e) = state.save_edited_snippet() {
                            return Err(e);
                        }
                        should_refresh = true;
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => {
                        state.edit_buffer.push(c);
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        state.edit_buffer.pop();
                    }
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        // Would need cursor position tracking for proper implementation
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        // Would need cursor position tracking for proper implementation
                    }
                    _ => {}
                },
                InputMode::Confirming => match key {
                    KeyEvent {
                        code: KeyCode::Char('y'),
                        ..
                    } => {
                        // Handle confirmation
                        if let Some(ConfirmAction::Delete) = state.confirm_action {
                            if let Some(actual_index) = state.get_selected_entry_index() {
                                let shortcut = state.entries[actual_index].shortcut.clone();
                                if let Err(e) = delete_snippet(&shortcut) {
                                    return Err(e);
                                }

                                // Reload entries and update state safely
                                match load_snippets() {
                                    Ok(entries) => {
                                        state.update_entries(entries);
                                    }
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                        state.input_mode = InputMode::Normal;
                        state.confirm_action = None;
                    }
                    KeyEvent {
                        code: KeyCode::Char('n'),
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        // Cancel confirmation
                        state.input_mode = InputMode::Normal;
                        state.confirm_action = None;
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
            // For backward compatibility
            if let Some(actual_index) = state.get_selected_entry_index() {
                let shortcut = state.entries[actual_index].shortcut.clone();
                if let Err(e) = delete_snippet(&shortcut) {
                    return Err(e);
                }

                // Reload entries safely
                match load_snippets() {
                    Ok(entries) => {
                        state.update_entries(entries);
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

    //  Handle empty filtered list correctly
    if state.filtered_indices.is_empty() {
        let list = List::new(vec![ListItem::new("No snippets found")])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Snippets (0) "),
            )
            .style(Style::default().fg(Color::Gray));

        f.render_widget(list, area);
        return;
    }

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
            Span::styled("  e", Style::default().fg(Color::Green)),
            Span::raw(": Edit selected snippet"),
        ]),
        Line::from(vec![
            Span::styled("  d", Style::default().fg(Color::Green)),
            Span::raw(": Delete selected snippet"),
        ]),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Green)),
            Span::raw(": Search snippets"),
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
        Line::from("• Or interactively with: scribe new"),
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

fn render_confirmation_dialog<B: ratatui::backend::Backend>(
    f: &mut Frame<B>,
    state: &AppState,
    size: Rect,
) {
    let message = match state.confirm_action {
        Some(ConfirmAction::Delete) => {
            if let Some(actual_index) = state.get_selected_entry_index() {
                let shortcut = &state.entries[actual_index].shortcut;
                format!("Delete snippet '{}' (y/n)?", shortcut)
            } else {
                "Delete selected snippet (y/n)?".to_string()
            }
        }
        None => "Confirm action (y/n)?".to_string(),
    };

    // Create a small centered dialog box
    let dialog_width = message.len() as u16 + 10;
    let dialog_height = 5;
    let dialog_x = (size.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (size.height.saturating_sub(dialog_height)) / 2;

    let dialog_rect = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    let dialog = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray))
        .title(" Confirm ");

    let text = Paragraph::new(message)
        .block(dialog)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(Clear, dialog_rect); // Clear the area
    f.render_widget(text, dialog_rect);
}

fn render_status_bar(state: &AppState) -> Paragraph<'static> {
    let help_text = match state.get_current_tab() {
        "Snippets" => match state.input_mode {
            InputMode::Normal => {
                "↑↓:Navigate | Enter:Copy | e:Edit | d:Delete | /:Search | Tab:Switch | Esc/q:Exit"
            }
            InputMode::Filtering => "Enter:Apply Filter | Esc:Cancel",
            InputMode::Editing => "Enter:Save | Esc:Cancel",
            InputMode::Confirming => "y:Yes | n/Esc:No",
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
