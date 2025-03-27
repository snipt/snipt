use crate::config::is_daemon_running;
use crate::error::{Result, ScribeError};
use crate::interactive_add;
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
use std::thread;
use std::time::Duration;

/// Display the main Scribe dashboard UI
pub fn display_scribe_dashboard(daemon_status: Option<u32>) -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create dashboard state
    let mut dashboard_state = DashboardState {
        daemon_status,
        selected_action: 0,
        exiting: false,
    };

    let result = run_dashboard(&mut terminal, &mut dashboard_state);

    // Clean up terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

struct DashboardState {
    daemon_status: Option<u32>,
    selected_action: usize,
    exiting: bool,
}

fn run_dashboard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut DashboardState,
) -> Result<()> {
    // List of available actions
    let actions = vec![
        "Manage Snippets",
        "Add New Snippet",
        "Start Daemon",
        "Stop Daemon",
        "Exit",
    ];

    // Initial draw to prevent flickering on first render
    terminal.draw(|_| {})?;

    while !state.exiting {
        // Only draw when needed, not on every loop iteration
        terminal.draw(|f| {
            let size = f.size();

            // Create main layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title area
                    Constraint::Length(3), // Status area
                    Constraint::Min(10),   // Actions area
                    Constraint::Length(1), // Help text
                ])
                .split(size);

            // Draw title
            let title = Paragraph::new("Scribe - Text Expansion Tool")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, main_chunks[0]);

            // Draw daemon status
            let status_text = match state.daemon_status {
                Some(pid) => {
                    vec![
                        Span::styled("Status: ", Style::default().fg(Color::White)),
                        Span::styled("● ", Style::default().fg(Color::Green)), // Green dot
                        Span::styled("ONLINE", Style::default().fg(Color::Green)),
                        Span::styled(
                            format!(" (PID: {})", pid),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]
                }
                None => {
                    vec![
                        Span::styled("Status: ", Style::default().fg(Color::White)),
                        Span::styled("● ", Style::default().fg(Color::Red)), // Red dot
                        Span::styled("OFFLINE", Style::default().fg(Color::Red)),
                    ]
                }
            };

            let status = Paragraph::new(Line::from(status_text))
                .style(Style::default())
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Daemon Status "),
                );
            f.render_widget(status, main_chunks[1]);

            // Draw actions
            let action_items: Vec<ListItem> = actions
                .iter()
                .enumerate()
                .map(|(i, &action)| {
                    let content = Line::from(vec![
                        if i == state.selected_action {
                            Span::styled("> ", Style::default().fg(Color::Yellow))
                        } else {
                            Span::raw("  ")
                        },
                        Span::styled(action, Style::default().fg(Color::White)),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let actions_list = List::new(action_items)
                .block(Block::default().borders(Borders::ALL).title(" Actions "))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            f.render_widget(actions_list, main_chunks[2]);

            // Draw help text
            let help_text = "↑/↓: Navigate | Enter: Select | q: Quit";
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(help, main_chunks[3]);
        })?;

        // Handle input with a timeout to prevent excessive CPU usage
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if state.selected_action > 0 {
                            state.selected_action -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if state.selected_action < actions.len() - 1 {
                            state.selected_action += 1;
                        }
                    }
                    KeyCode::Enter => {
                        match state.selected_action {
                            0 => {
                                // Manage Snippets - Clean approach
                                // First exit TUI mode properly
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                // Run the snippet manager in normal terminal mode
                                let result = display_snippet_manager();

                                // Regardless of result, restore our TUI
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                terminal.clear()?;

                                // If there was an error, show it briefly
                                if let Err(e) = result {
                                    show_message(
                                        terminal,
                                        &format!("Error: {}", e),
                                        Color::Red,
                                        2000,
                                    )?;
                                }

                                // Update daemon status
                                state.daemon_status = is_daemon_running()?;
                            }
                            1 => {
                                // Add New Snippet - Clean approach
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                // Run the interactive add function
                                let result = interactive_add();

                                // Restore TUI
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                terminal.clear()?;

                                // Show error if needed
                                if let Err(e) = result {
                                    show_message(
                                        terminal,
                                        &format!("Error: {}", e),
                                        Color::Red,
                                        2000,
                                    )?;
                                }
                            }
                            2 => {
                                // Start Daemon
                                if state.daemon_status.is_none() {
                                    // Show starting message within the UI
                                    show_message(
                                        terminal,
                                        "Starting daemon process...",
                                        Color::Yellow,
                                        500,
                                    )?;

                                    // We'll need to temporarily disable raw mode but remember we're in a UI
                                    disable_raw_mode()?;

                                    // Start daemon using a separate process that won't detach our terminal
                                    let output =
                                        std::process::Command::new(std::env::current_exe()?)
                                            .arg("start")
                                            .output();

                                    // Re-enable raw mode for our UI
                                    enable_raw_mode()?;

                                    // Process the result
                                    match output {
                                        Ok(output) => {
                                            // Give the daemon a moment to start
                                            thread::sleep(Duration::from_millis(1000));

                                            // Check if daemon is actually running
                                            let is_running = is_daemon_running()?;
                                            if is_running.is_some() {
                                                show_message(
                                                    terminal,
                                                    "Daemon started successfully",
                                                    Color::Green,
                                                    1000,
                                                )?;
                                                state.daemon_status = is_running;
                                            } else {
                                                // Something went wrong - show the error output
                                                let stderr =
                                                    String::from_utf8_lossy(&output.stderr);
                                                let error_msg = if !stderr.is_empty() {
                                                    format!("Daemon failed to start: {}", stderr)
                                                } else {
                                                    "Daemon failed to start for unknown reason"
                                                        .to_string()
                                                };

                                                show_message(
                                                    terminal,
                                                    &error_msg,
                                                    Color::Red,
                                                    2000,
                                                )?;
                                            }
                                        }
                                        Err(e) => {
                                            show_message(
                                                terminal,
                                                &format!("Failed to start daemon process: {}", e),
                                                Color::Red,
                                                2000,
                                            )?;
                                        }
                                    }
                                } else {
                                    // Show "already running" message
                                    show_message(
                                        terminal,
                                        "Daemon is already running",
                                        Color::Yellow,
                                        1000,
                                    )?;
                                }
                            }

                            // For the Stop Daemon action (case 3):
                            3 => {
                                // Stop Daemon
                                if state.daemon_status.is_some() {
                                    // Show stopping message
                                    show_message(
                                        terminal,
                                        "Stopping daemon process...",
                                        Color::Yellow,
                                        500,
                                    )?;

                                    // Run the stop command as a separate process
                                    disable_raw_mode()?;

                                    let output =
                                        std::process::Command::new(std::env::current_exe()?)
                                            .arg("stop")
                                            .output();

                                    enable_raw_mode()?;

                                    // Process the result
                                    match output {
                                        Ok(_) => {
                                            // Give the daemon a moment to stop
                                            thread::sleep(Duration::from_millis(500));

                                            // Verify that daemon is actually stopped
                                            let is_running = is_daemon_running()?;
                                            if is_running.is_none() {
                                                show_message(
                                                    terminal,
                                                    "Daemon stopped successfully",
                                                    Color::Green,
                                                    1000,
                                                )?;
                                                state.daemon_status = None;
                                            } else {
                                                show_message(
                                                    terminal,
                                                    "Failed to stop daemon - process is still running",
                                                    Color::Red,
                                                    2000
                                                )?;
                                                state.daemon_status = is_running;
                                                // Update with actual status
                                            }
                                        }
                                        Err(e) => {
                                            show_message(
                                                terminal,
                                                &format!("Failed to stop daemon process: {}", e),
                                                Color::Red,
                                                2000,
                                            )?;
                                        }
                                    }
                                } else {
                                    // Show "not running" message
                                    show_message(
                                        terminal,
                                        "Daemon is not running",
                                        Color::Yellow,
                                        1000,
                                    )?;
                                }
                            }
                            4 => {
                                // Exit
                                state.exiting = true;
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Char('q') => {
                        state.exiting = true;
                    }
                    KeyCode::Esc => {
                        state.exiting = true;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// Helper function to show messages in a popup
fn show_message<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    message: &str,
    color: Color,
    duration_ms: u64,
) -> Result<()> {
    // Draw the message
    terminal.draw(|f| {
        let size = f.size();
        let area = centered_rect(60, 10, size);

        // Clear the area behind the popup
        f.render_widget(Clear, area);

        // Create the message box - add instructions if it's a wait
        let message_text = if duration_ms == 0 {
            format!("{}\n\nPress any key to continue...", message)
        } else {
            message.to_string()
        };

        let message_box = Paragraph::new(message_text)
            .style(Style::default().fg(color))
            .block(Block::default().borders(Borders::ALL).title(" Scribe "))
            .alignment(Alignment::Center);

        f.render_widget(message_box, area);
    })?;

    // If duration is specified, wait that long
    if duration_ms > 0 {
        // Sleep but still be interruptible by key press
        for _ in 0..duration_ms / 100 {
            thread::sleep(Duration::from_millis(100));
            if crossterm::event::poll(Duration::from_millis(0))? {
                let _ = crossterm::event::read()?;
                break;
            }
        }
    } else {
        // Wait for a key press to dismiss with a timeout
        if crossterm::event::poll(Duration::from_secs(30))? {
            let _ = crossterm::event::read()?;
        }
    }

    Ok(())
}

// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

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
    edit_buffer: Vec<String>, // Changed to Vec<String> for multiline editing
    edit_cursor_pos: usize,   // Cursor position in the current line
    edit_line: usize,         // Current line being edited
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
            edit_buffer: vec![String::new()],
            edit_cursor_pos: 0,
            edit_line: 0,
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

    fn get_selected_entry(&self) -> Option<&SnippetEntry> {
        self.get_filtered_entry(self.selected)
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
            // Split the snippet into lines
            let lines: Vec<String> = self.entries[actual_index]
                .snippet
                .split('\n')
                .map(|s| s.to_string())
                .collect();

            self.edit_buffer = if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            };

            self.edit_cursor_pos = 0;
            self.edit_line = 0;
            self.input_mode = InputMode::Editing;
        }
    }

    fn save_edited_snippet(&mut self) -> Result<()> {
        if let Some(actual_index) = self.get_selected_entry_index() {
            let shortcut = self.entries[actual_index].shortcut.clone();
            let new_snippet = self.edit_buffer.join("\n");

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

    // Update entries safely after deletion or any other operation
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
                    // Show current line being edited and line/cursor position info
                    let line_count = state.edit_buffer.len();
                    let edit_info = format!(
                        "Line {}/{} | Ctrl+w to save",
                        state.edit_line + 1,
                        line_count
                    );

                    let edit_text = format!("Edit: {}", state.edit_buffer[state.edit_line]);
                    let edit = Paragraph::new(edit_text)
                        .style(Style::default().fg(Color::Green))
                        .alignment(Alignment::Left);

                    // Split the edit area to show both the current line and position info
                    let edit_area_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                        .split(main_chunks[2]);

                    f.render_widget(edit, edit_area_chunks[0]);

                    let info = Paragraph::new(edit_info)
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Right);
                    f.render_widget(info, edit_area_chunks[1]);
                }
                InputMode::Confirming => {
                    // Don't change the filter area during confirmation
                }
            }

            // Render confirmation dialog if needed
            if state.input_mode == InputMode::Confirming {
                render_confirmation_dialog(f, state, size);
            }

            // Render multiline editor if in editing mode
            if state.input_mode == InputMode::Editing {
                draw_multiline_editor(f, state, size);
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
                        state.edit_buffer.push(String::new());
                        state.edit_line = 0;
                        state.edit_cursor_pos = 0;
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        // Always insert a new line
                        let current_line = &state.edit_buffer[state.edit_line];
                        let rest_of_line = if state.edit_cursor_pos < current_line.len() {
                            current_line[state.edit_cursor_pos..].to_string()
                        } else {
                            String::new()
                        };

                        state.edit_buffer[state.edit_line].truncate(state.edit_cursor_pos);
                        state.edit_buffer.insert(state.edit_line + 1, rest_of_line);
                        state.edit_line += 1;
                        state.edit_cursor_pos = 0;
                    }
                    KeyEvent {
                        code: KeyCode::Char('w'),
                        modifiers,
                        ..
                    } if modifiers.contains(KeyModifiers::CONTROL) => {
                        // Save with Ctrl+w
                        if let Err(e) = state.save_edited_snippet() {
                            return Err(e);
                        }
                        should_refresh = true;
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => {
                        state.edit_buffer[state.edit_line].insert(state.edit_cursor_pos, c);
                        state.edit_cursor_pos += 1;
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        if state.edit_cursor_pos > 0 {
                            state.edit_buffer[state.edit_line].remove(state.edit_cursor_pos - 1);
                            state.edit_cursor_pos -= 1;
                        } else if state.edit_line > 0 {
                            // At start of line, merge with previous line
                            let current_content = state.edit_buffer.remove(state.edit_line);
                            state.edit_line -= 1;
                            state.edit_cursor_pos = state.edit_buffer[state.edit_line].len();
                            state.edit_buffer[state.edit_line].push_str(&current_content);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Delete,
                        ..
                    } => {
                        if state.edit_cursor_pos < state.edit_buffer[state.edit_line].len() {
                            state.edit_buffer[state.edit_line].remove(state.edit_cursor_pos);
                        } else if state.edit_line < state.edit_buffer.len() - 1 {
                            // At end of line, merge with next line
                            let next_content = state.edit_buffer.remove(state.edit_line + 1);
                            state.edit_buffer[state.edit_line].push_str(&next_content);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        if state.edit_cursor_pos > 0 {
                            state.edit_cursor_pos -= 1;
                        } else if state.edit_line > 0 {
                            // Move to end of previous line
                            state.edit_line -= 1;
                            state.edit_cursor_pos = state.edit_buffer[state.edit_line].len();
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        if state.edit_cursor_pos < state.edit_buffer[state.edit_line].len() {
                            state.edit_cursor_pos += 1;
                        } else if state.edit_line < state.edit_buffer.len() - 1 {
                            // Move to start of next line
                            state.edit_line += 1;
                            state.edit_cursor_pos = 0;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => {
                        if state.edit_line > 0 {
                            state.edit_line -= 1;
                            state.edit_cursor_pos = state
                                .edit_cursor_pos
                                .min(state.edit_buffer[state.edit_line].len());
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => {
                        if state.edit_line < state.edit_buffer.len() - 1 {
                            state.edit_line += 1;
                            state.edit_cursor_pos = state
                                .edit_cursor_pos
                                .min(state.edit_buffer[state.edit_line].len());
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Home,
                        ..
                    } => {
                        state.edit_cursor_pos = 0;
                    }
                    KeyEvent {
                        code: KeyCode::End, ..
                    } => {
                        state.edit_cursor_pos = state.edit_buffer[state.edit_line].len();
                    }
                    KeyEvent {
                        code: KeyCode::Tab, ..
                    } => {
                        // Insert 4 spaces for indentation
                        for _ in 0..4 {
                            state.edit_buffer[state.edit_line].insert(state.edit_cursor_pos, ' ');
                            state.edit_cursor_pos += 1;
                        }
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

            // Extract just the first line for preview
            let preview_content = entry.snippet.lines().next().unwrap_or("").to_string();
            let snippet_preview = if preview_content.len() > 20 {
                format!("{}...", &preview_content[..17])
            } else {
                preview_content
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

        // Create text to display multiline snippet with proper indentation
        let mut content = vec![
            shortcut_line,
            timestamp_line,
            Line::from(""),
            Line::from(snippet_label),
        ];

        // Split the snippet content by newlines and preserve indentation
        for line in entry.snippet.split('\n') {
            content.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }

        // Calculate how many lines we can show in the available space
        let available_lines = area.height.saturating_sub(7) as usize; // Adjust for borders, headers, etc.

        // If we have more lines than can fit, add an indicator
        if content.len() > available_lines + 4 {
            // +4 for the header lines
            content.truncate(available_lines + 4);
            content.push(Line::from(Span::styled(
                "... (more lines not shown) ...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(Text::from(content))
            .block(block)
            .wrap(Wrap { trim: false }); // Don't trim whitespace for indentation

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No snippet selected")
            .block(block)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}

fn draw_multiline_editor<B: ratatui::backend::Backend>(
    f: &mut Frame<B>,
    state: &AppState,
    size: Rect,
) {
    // Calculate editor dimensions
    let width = size.width.min(80).max(40);
    let height = size.height.min(20).max(10);
    let x = (size.width - width) / 2;
    let y = (size.height - height) / 2;

    let editor_rect = Rect {
        x,
        y,
        width,
        height,
    };

    // Clear the area behind the editor
    f.render_widget(Clear, editor_rect);

    // Draw editor border
    let editor_title = format!(
        " Editing Snippet: {} ",
        state.get_selected_entry().map_or("", |e| &e.shortcut)
    );

    let editor_block = Block::default()
        .borders(Borders::ALL)
        .title(editor_title)
        .style(Style::default().bg(Color::Black));

    // Calculate inner area for text content
    let inner_area = editor_block.inner(editor_rect);
    f.render_widget(editor_block, editor_rect);

    let visible_height = inner_area.height as usize;

    // Calculate scroll offset to keep the cursor line in view
    let scroll_offset = if state.edit_line >= visible_height {
        state.edit_line - visible_height + 1
    } else {
        0
    };
    let visible_content: Vec<Line> = state
        .edit_buffer
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, line)| {
            let line_number = Span::styled(
                format!("{:3} ", idx + 1),
                Style::default().fg(Color::DarkGray),
            );

            let line_content = if idx == state.edit_line {
                // Highlight current line
                Span::styled(line, Style::default().fg(Color::White).bg(Color::Blue))
            } else {
                Span::styled(line, Style::default().fg(Color::White))
            };

            Line::from(vec![line_number, line_content])
        })
        .collect();

    // Render the lines
    let text = Paragraph::new(visible_content)
        .scroll((0, scroll_offset as u16))
        .wrap(Wrap { trim: false }); // Don't trim whitespace to preserve indentation

    f.render_widget(text, inner_area);

    // Render help text at the bottom
    let help_text =
        "↑↓: Navigate Lines | Tab: Indent | Enter: New Line | Ctrl+w: Save | Esc: Cancel";
    let help_area = Rect {
        x,
        y: y + height,
        width,
        height: 1,
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, help_area);

    // Manually position the cursor
    // Since we're using a custom editor, we need to calculate where the cursor should be
    let cursor_x = inner_area.x + 4 + state.edit_cursor_pos as u16; // +4 for line number width
    let cursor_y = inner_area.y + (state.edit_line - scroll_offset) as u16;

    if cursor_y >= inner_area.y && cursor_y < inner_area.y + inner_area.height {
        f.set_cursor(cursor_x, cursor_y);
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
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("Multiline Snippets", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        ]),
                        Line::from("• Scribe supports multiline snippets for code, templates, and formatted text"),
                        Line::from("• When editing a snippet, use Enter for new lines and Tab for indentation"),
                        Line::from("• Save your edits with Ctrl+w"),
                        Line::from("• Indentation and formatting are preserved when snippets are expanded"),
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
            InputMode::Editing => {
                "Ctrl+w:Save | Enter:New Line | Tab:Indent | ↑↓:Navigate Lines | Esc:Cancel"
            }
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
            Line::from("The interactive mode supports multiline snippets with proper indentation."),
            Line::from("Perfect for code snippets, templates, and formatted text."),
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
