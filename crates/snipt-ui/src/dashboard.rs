use crate::{
    editor::{interactive_add, AddResult},
    snippet_manager::display_snippet_manager,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};
use snipt_core::{is_daemon_running, load_snippets, Result};
use std::io::{self, stdout};
use std::thread;
use std::time::Duration;

struct DashboardState {
    daemon_status: Option<u32>,
    selected_action: usize,
    exiting: bool,
    snippet_count: usize,
}

/// Display the main snipt dashboard UI
pub fn display_snipt_dashboard(daemon_status: Option<u32>) -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Load snippet count
    let snippets = load_snippets().unwrap_or_default();
    let snippet_count = snippets.len();

    // Create dashboard state
    let mut dashboard_state = DashboardState {
        daemon_status,
        selected_action: 0,
        exiting: false,
        snippet_count,
    };

    let result = run_dashboard(&mut terminal, &mut dashboard_state);

    // Clean up terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

fn run_dashboard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut DashboardState,
) -> Result<()> {
    // List of available actions
    let actions = ["Manage Snippets", "Add New Snippet"];

    // Add frame limiter to reduce flickering and CPU usage
    let mut last_render = std::time::Instant::now();
    const RENDER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33); // ~30fps
    let mut force_render = true; // Force initial render

    // Initial draw to prevent flickering on first render
    terminal.draw(|_| {})?;

    while !state.exiting {
        // Only draw when needed (when state changes or enough time has passed)
        let now = std::time::Instant::now();
        if force_render || now.duration_since(last_render) >= RENDER_INTERVAL {
            terminal.draw(|f| {
                let size = f.size();

                // Create a centered layout with distinct sections
                let vertical_margin = (size.height.saturating_sub(22)) / 2; // Increased for ASCII art
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(vertical_margin), // Top centering space
                        Constraint::Length(2),               // Title area
                        Constraint::Length(6),               // ASCII art logo
                        Constraint::Length(12),              // Main content area
                        Constraint::Length(4),               // Help area
                        Constraint::Min(0),                  // Bottom centering space
                    ])
                    .split(size);

                // Premium color scheme
                let primary_color = Color::Rgb(120, 90, 180); // Rich purple
                let secondary_color = Color::Rgb(240, 180, 50); // Gold accent
                let text_color = Color::Rgb(220, 220, 235); // Soft white
                let dark_bg = Color::Rgb(25, 20, 40); // Deep dark purple-blue
                let detail_color = Color::Rgb(100, 130, 190); // Steel blue
                let success_color = Color::Rgb(95, 215, 140); // Emerald green
                let error_color = Color::Rgb(235, 85, 85); // Soft red

                // Title block with version and snippet count - premium styling
                let title_block = Block::default()
                    .title(" snipt ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(secondary_color));

                // Get inner area before rendering the block
                let inner_title = title_block.inner(main_chunks[1]);
                f.render_widget(title_block, main_chunks[1]);

                let title_info = Line::from(vec![
                    Span::styled(
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled("  •  ", Style::default().fg(secondary_color)),
                    Span::styled(
                        format!("{} snippets stored", state.snippet_count),
                        Style::default().fg(text_color),
                    ),
                ]);

                let title_paragraph = Paragraph::new(title_info).alignment(Alignment::Center);
                f.render_widget(title_paragraph, inner_title);

                // ASCII art logo with premium colors
                let logo = [
                    "  ██████  ███▄    █  ██▓ ██▓███  ▄▄▄█████▓",
                    "▒██    ▒  ██ ▀█   █ ▓██▒▓██░  ██▒▓  ██▒ ▓▒",
                    "░ ▓██▄   ▓██  ▀█ ██▒▒██▒▓██░ ██▓▒▒ ▓██░ ▒░",
                    "  ▒   ██▒▓██▒  ▐▌██▒░██░▒██▄█▓▒ ▒░ ▓██▓ ░ ",
                    "▒██████▒▒▒██░   ▓██░░██░▒██▒ ░  ░  ▒██▒ ░ ",
                ];

                let logo_text: Vec<Line> = logo
                    .iter()
                    .map(|line| Line::from(Span::styled(*line, Style::default().fg(primary_color))))
                    .collect();

                let logo_widget = Paragraph::new(logo_text).alignment(Alignment::Center);
                f.render_widget(logo_widget, main_chunks[2]);

                // Main content area with premium styling
                let content_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(primary_color));

                // Get inner area before rendering the block
                let inner_content = content_block.inner(main_chunks[3]);
                f.render_widget(content_block, main_chunks[3]);

                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(45), // Status area
                        Constraint::Percentage(55), // Actions area
                    ])
                    .split(inner_content);

                // Status panel with daemon info - premium styling
                let status_block = Block::default()
                    .title(" System Status ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(secondary_color))
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(primary_color));

                let inner_status = status_block.inner(content_chunks[0]);
                f.render_widget(status_block, content_chunks[0]);

                let status_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2), // Status header
                        Constraint::Length(7), // Status details
                    ])
                    .split(inner_status);

                // Status header with daemon state - premium styling
                let status_header = match state.daemon_status {
                    Some(_) => vec![Line::from(vec![
                        Span::styled("● ", Style::default().fg(success_color)),
                        Span::styled(
                            "DAEMON RUNNING",
                            Style::default()
                                .fg(success_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])],
                    None => vec![Line::from(vec![
                        Span::styled("● ", Style::default().fg(error_color)),
                        Span::styled(
                            "DAEMON STOPPED",
                            Style::default()
                                .fg(error_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])],
                };

                let header_paragraph = Paragraph::new(status_header).alignment(Alignment::Center);
                f.render_widget(header_paragraph, status_layout[0]);

                // Enhanced status details with premium styling
                let status_text = match state.daemon_status {
                    Some(pid) => {
                        vec![
                            Line::from(vec![
                                Span::styled("Process ID:     ", Style::default().fg(detail_color)),
                                Span::styled(format!("{}", pid), Style::default().fg(text_color)),
                            ]),
                            Line::from(vec![
                                Span::styled("Text expansion: ", Style::default().fg(detail_color)),
                                Span::styled("Active", Style::default().fg(success_color)),
                            ]),
                            Line::from(vec![
                                Span::styled("Trigger method: ", Style::default().fg(detail_color)),
                                Span::styled(
                                    "Type shortcut + space/tab/enter",
                                    Style::default().fg(text_color),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::styled(
                                    "Control command: ",
                                    Style::default().fg(detail_color),
                                ),
                                Span::styled("snipt stop", Style::default().fg(secondary_color)),
                            ]),
                        ]
                    }
                    None => {
                        vec![
                            Line::from(vec![
                                Span::styled("Text expansion: ", Style::default().fg(detail_color)),
                                Span::styled("Inactive", Style::default().fg(error_color)),
                            ]),
                            Line::from(vec![
                                Span::styled("Status:        ", Style::default().fg(detail_color)),
                                Span::styled(
                                    "Snippets won't expand",
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]),
                            Line::from(vec![
                                Span::styled("Required:      ", Style::default().fg(detail_color)),
                                Span::styled(
                                    "Daemon must be running for expansion",
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::styled(
                                    "Control command: ",
                                    Style::default().fg(detail_color),
                                ),
                                Span::styled("snipt start", Style::default().fg(secondary_color)),
                            ]),
                        ]
                    }
                };

                let status_paragraph = Paragraph::new(status_text).alignment(Alignment::Left);
                f.render_widget(status_paragraph, status_layout[1]);

                // Actions panel with premium styling
                let action_block = Block::default()
                    .title(" Actions ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(secondary_color))
                    .borders(Borders::NONE);

                let inner_action = action_block.inner(content_chunks[1]);
                f.render_widget(action_block, content_chunks[1]);

                // Action style definitions with premium styling
                let action_styles = [
                    (
                        secondary_color,
                        "📁",
                        "Manage Snippets",
                        "Browse, edit and manage your snippet collection",
                    ),
                    (
                        secondary_color,
                        "✨",
                        "Add New Snippet",
                        "Create a new text expansion snippet",
                    ),
                ];

                // Action buttons layout
                let button_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .vertical_margin(1)
                    .constraints([
                        Constraint::Length(3), // First button
                        Constraint::Length(1), // Spacer
                        Constraint::Length(3), // Second button
                    ])
                    .split(inner_action);

                // Draw each button with premium styling
                for (i, _) in actions.iter().enumerate() {
                    let is_selected = i == state.selected_action;
                    let (color, icon, title, description) = action_styles[i];

                    // Select the button area
                    let button_area = match i {
                        0 => button_chunks[0],
                        _ => button_chunks[2],
                    };

                    // Create border around the selected button - premium styling
                    let button_block = Block::default()
                        .borders(if is_selected {
                            Borders::ALL
                        } else {
                            Borders::NONE
                        })
                        .border_style(Style::default().fg(color))
                        .style(Style::default().bg(if is_selected {
                            dark_bg
                        } else {
                            Color::Reset
                        }));

                    // Get inner area before rendering the block
                    let inner_button = button_block.inner(button_area);
                    f.render_widget(button_block, button_area);

                    // Indicator and styling with premium feel
                    let select_indicator = if is_selected { "›" } else { " " };

                    // Main action title - premium styling
                    let action_title = Line::from(vec![Span::styled(
                        format!(" {} {} {}", select_indicator, icon, title),
                        Style::default()
                            .fg(if is_selected {
                                secondary_color
                            } else {
                                detail_color
                            })
                            .add_modifier(Modifier::BOLD),
                    )]);

                    // Action description with premium styling
                    let action_desc = Line::from(vec![Span::styled(
                        format!("    {}", description),
                        Style::default().fg(if is_selected {
                            text_color
                        } else {
                            Color::DarkGray
                        }),
                    )]);

                    let button_content = vec![action_title, action_desc];
                    let button_paragraph = Paragraph::new(button_content);
                    f.render_widget(button_paragraph, inner_button);
                }

                // Help section with premium styling
                let help_block = Block::default()
                    .title(" Help & Tips ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(secondary_color))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(primary_color));

                // Get inner area before rendering the block
                let inner_help = help_block.inner(main_chunks[4]);
                f.render_widget(help_block, main_chunks[4]);

                let help_text = vec![
                    Line::from(vec![
                        Span::styled(
                            "Navigation: ",
                            Style::default()
                                .fg(detail_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("[↑/↓]", Style::default().fg(secondary_color)),
                        Span::raw(" select  "),
                        Span::styled("[Enter]", Style::default().fg(secondary_color)),
                        Span::raw(" choose  "),
                        Span::styled("[Esc/q]", Style::default().fg(secondary_color)),
                        Span::raw(" exit"),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            "Usage: ",
                            Style::default()
                                .fg(detail_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("Snippets activate when you type their shortcut followed by "),
                        Span::styled("space", Style::default().fg(secondary_color)),
                        Span::raw(", "),
                        Span::styled("tab", Style::default().fg(secondary_color)),
                        Span::raw(" or "),
                        Span::styled("enter", Style::default().fg(secondary_color)),
                    ]),
                ];

                let help_paragraph = Paragraph::new(help_text).alignment(Alignment::Center);
                f.render_widget(help_paragraph, inner_help);
            })?;

            last_render = now;
            force_render = false;
        }

        // Poll for user input with a short timeout
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if state.selected_action > 0 {
                            state.selected_action -= 1;
                            force_render = true;
                        }
                    }
                    KeyCode::Down => {
                        if state.selected_action < actions.len() - 1 {
                            state.selected_action += 1;
                            force_render = true;
                        }
                    }
                    KeyCode::Enter => {
                        force_render = true;
                        match state.selected_action {
                            0 => {
                                // Manage Snippets
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                // Run the snippet manager
                                let result = display_snippet_manager();

                                // Restore TUI
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                terminal.clear()?;

                                // Handle errors
                                if let Err(e) = result {
                                    show_message(
                                        terminal,
                                        &format!("Error: {}", e),
                                        Color::Red,
                                        2000,
                                    )?;
                                }

                                // Update state information
                                state.daemon_status = is_daemon_running()?;
                                let snippets = load_snippets().unwrap_or_default();
                                state.snippet_count = snippets.len();
                            }
                            1 => {
                                // Add New Snippet
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                // Run the add snippet editor
                                match interactive_add() {
                                    AddResult::Added => {
                                        // Success - show message
                                        enable_raw_mode()?;
                                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                        show_message(
                                            terminal,
                                            "Snippet added successfully!",
                                            Color::Green,
                                            2000,
                                        )?;

                                        // Update state information
                                        state.daemon_status = is_daemon_running()?;
                                        let snippets = load_snippets().unwrap_or_default();
                                        state.snippet_count = snippets.len();

                                        // Exit this process
                                        return Ok(());
                                    }
                                    AddResult::Cancelled => {
                                        // User canceled - restore dashboard
                                        state.daemon_status = is_daemon_running()?;
                                        disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                        // Run the snippet manager
                                        let result = display_snipt_dashboard(state.daemon_status);

                                        // Restore TUI
                                        enable_raw_mode()?;
                                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                        terminal.clear()?;

                                        // Handle errors
                                        if let Err(e) = result {
                                            show_message(
                                                terminal,
                                                &format!("Error: {}", e),
                                                Color::Red,
                                                2000,
                                            )?;
                                        }

                                        // Update state information
                                        state.daemon_status = is_daemon_running()?;
                                        let snippets = load_snippets().unwrap_or_default();
                                        state.snippet_count = snippets.len();
                                    }
                                    AddResult::Error(e) => {
                                        // Error - restore dashboard with error message
                                        eprintln!("Error: {}", e);
                                        enable_raw_mode()?;
                                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                        show_message(
                                            terminal,
                                            &format!("Error: {}", e),
                                            Color::Red,
                                            2000,
                                        )?;
                                    }
                                }

                                // Update state for dashboard
                                state.daemon_status = is_daemon_running()?;
                                force_render = true;
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
            .block(Block::default().borders(Borders::ALL).title(" snipt "))
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
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
