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
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Terminal,
};
use snipt_core::{is_daemon_running, Result};
use std::io::{self, stdout};
use std::thread;
use std::time::Duration;

struct DashboardState {
    daemon_status: Option<u32>,
    selected_action: usize,
    exiting: bool,
}

/// Display the main snipt dashboard UI
pub fn display_snipt_dashboard(daemon_status: Option<u32>) -> Result<()> {
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

fn run_dashboard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut DashboardState,
) -> Result<()> {
    // List of available actions
    let actions = vec!["Manage Snippets", "Add New Snippet"];

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

                // Create main layout
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // Title area
                        Constraint::Length(10), // Logo/banner area
                        Constraint::Length(6),  // Status area with start/stop instructions
                        Constraint::Min(8),     // Actions area
                        Constraint::Length(2),  // Help text
                    ])
                    .split(size);

                // Draw title with version
                let version = env!("CARGO_PKG_VERSION");
                let title = Paragraph::new(format!("snipt v{}", version))
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(title, main_chunks[0]);

                // ASCII art logo/banner for visual appeal
                let logo = vec![
                    "  ██████  ███▄    █  ██▓ ██▓███  ▄▄▄█████▓",
                    "▒██    ▒  ██ ▀█   █ ▓██▒▓██░  ██▒▓  ██▒ ▓▒",
                    "░ ▓██▄   ▓██  ▀█ ██▒▒██▒▓██░ ██▓▒▒ ▓██░ ▒░",
                    "  ▒   ██▒▓██▒  ▐▌██▒░██░▒██▄█▓▒ ▒░ ▓██▓ ░ ",
                    "▒██████▒▒▒██░   ▓██░░██░▒██▒ ░  ░  ▒██▒ ░ ",
                    "▒ ▒▓▒ ▒ ░░ ▒░   ▒ ▒ ░▓  ▒▓▒░ ░  ░  ▒ ░░   ",
                    "░ ░▒  ░ ░░ ░░   ░ ▒░ ▒ ░░▒ ░         ░    ",
                ];

                let logo_text: Vec<Line> = logo
                    .iter()
                    .map(|line| {
                        Line::from(Span::styled(
                            *line, // Use *line instead of line to dereference it
                            Style::default().fg(Color::Magenta),
                        ))
                    })
                    .collect();

                let logo_widget = Paragraph::new(logo_text).alignment(Alignment::Center);
                f.render_widget(logo_widget, main_chunks[1]);

                // Draw daemon status with clear start/stop instructions
                let status_text = match state.daemon_status {
                    Some(pid) => {
                        vec![
                            Line::from(vec![
                                Span::styled("Status: ", Style::default().fg(Color::White)),
                                Span::styled("● ", Style::default().fg(Color::Green)), // Green dot
                                Span::styled("RUNNING", Style::default().fg(Color::Green)),
                                Span::styled(
                                    format!(" (PID: {})", pid),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::raw("To "),
                                Span::styled("stop", Style::default().fg(Color::Red)),
                                Span::raw(" the daemon, run: "),
                                Span::styled("snipt stop", Style::default().fg(Color::Yellow)),
                            ]),
                        ]
                    }
                    None => {
                        vec![
                            Line::from(vec![
                                Span::styled("Status: ", Style::default().fg(Color::White)),
                                Span::styled("● ", Style::default().fg(Color::Red)), // Red dot
                                Span::styled("STOPPED", Style::default().fg(Color::Red)),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::raw("To "),
                                Span::styled("start", Style::default().fg(Color::Green)),
                                Span::raw(" the daemon, run: "),
                                Span::styled("snipt start", Style::default().fg(Color::Yellow)),
                            ]),
                        ]
                    }
                };

                // Add explanation of daemon's purpose
                let daemon_explanation = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
                        Span::raw("The daemon must be running for text expansion to work"),
                    ]),
                ];

                let mut all_status_text = status_text;
                all_status_text.extend(daemon_explanation);

                let status = Paragraph::new(all_status_text)
                    .style(Style::default())
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Daemon Status "),
                    );
                f.render_widget(status, main_chunks[2]);

                let action_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2), // Top gap
                        Constraint::Min(8),    // Action list area
                    ])
                    .split(main_chunks[3]);

                // Calculate dimensions for a well-spaced list
                let list_area = action_area[1];
                let available_width = list_area.width;
                let list_width = (available_width as f32 * 0.5).round() as u16; // 50% of width for better readability
                let list_x = (available_width - list_width) / 2; // Center horizontally

                let list_rect = Rect {
                    x: list_area.x + list_x,
                    y: list_area.y,
                    width: list_width,
                    height: (actions.len() * 3) as u16 + 2, // More height for spacing (3 rows per item + borders)
                };

                // Create list items with center alignment and larger text
                let items: Vec<ListItem> = actions
                    .iter()
                    .enumerate()
                    .map(|(i, &action)| {
                        let is_selected = i == state.selected_action;
                        let color = match i {
                            0 => Color::Cyan,
                            1 => Color::Green,
                            _ => Color::White,
                        };

                        // Calculate padding for centering text manually
                        let action_len = action.len();
                        let available_width = list_width as usize - 6; // Subtract some space for borders and indicator
                        let left_padding = (available_width.saturating_sub(action_len)) / 2;
                        let padding = " ".repeat(left_padding);

                        // Create styled line with larger text (using symbols for emphasis)
                        let prefix = if is_selected { ">" } else { " " };

                        // Use empty lines for better spacing between items
                        let item = ListItem::new(vec![
                            // Empty line above for spacing
                            Line::from(""),
                            // Main content line - centered manually with padding
                            Line::from(vec![
                                Span::raw(padding), // Add padding for centering
                                Span::styled(
                                    format!("{} ", prefix),
                                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    action,
                                    Style::default()
                                        .fg(if is_selected { color } else { Color::White })
                                        // Add bold for larger appearance
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]),
                            // Empty line below for spacing
                            Line::from(""),
                        ]);

                        item
                    })
                    .collect();

                let actions_list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::NONE) // Remove borders for cleaner look
                            .title_alignment(Alignment::Center),
                    )
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .style(Style::default()); // Base style

                f.render_widget(actions_list, list_rect);

                // Draw help text
                let help_text = "↑/↓: Navigate | Tab: Navigate | Enter: Select | q: Quit";
                let help = Paragraph::new(help_text)
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center);
                f.render_widget(help, main_chunks[4]);
            })?;

            last_render = now;
            force_render = false;
        }

        // Handle input with a timeout to prevent excessive CPU usage
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        if state.selected_action > 0 {
                            state.selected_action -= 1;
                            force_render = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        if state.selected_action < actions.len() - 1 {
                            state.selected_action += 1;
                            force_render = true;
                        } else {
                            // Wrap around to the first option when at the end
                            state.selected_action = 0;
                            force_render = true;
                        }
                    }
                    KeyCode::BackTab => {
                        // Shift+Tab moves backward
                        if state.selected_action > 0 {
                            state.selected_action -= 1;
                        } else {
                            // Wrap around to the last option when at the beginning
                            state.selected_action = actions.len() - 1;
                        }
                        force_render = true;
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

                                // Update daemon status
                                state.daemon_status = is_daemon_running()?;
                            }
                            1 => {
                                // Add New Snippet
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                // Run the interactive add function
                                let add_result = interactive_add();

                                // Reset terminal state
                                let _ = disable_raw_mode();
                                let _ = execute!(std::io::stdout(), LeaveAlternateScreen)?;

                                match add_result {
                                    AddResult::Added => {
                                        // Success - launch snippet manager
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

                                        // Update daemon status
                                        state.daemon_status = is_daemon_running()?;
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

                                        // Update daemon status
                                        state.daemon_status = is_daemon_running()?;
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
