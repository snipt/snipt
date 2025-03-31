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

                // Create main layout with better proportions
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Title area
                        Constraint::Length(5), // Logo area
                        Constraint::Length(5), // Status area
                        Constraint::Min(6),    // Actions area - reduced minimum height
                        Constraint::Length(2), // Help text
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

                // ASCII art logo - using a more compact version
                let logo = vec![
                    "  ██████  ███▄    █  ██▓ ██▓███  ▄▄▄█████▓",
                    "▒██    ▒  ██ ▀█   █ ▓██▒▓██░  ██▒▓  ██▒ ▓▒",
                    "░ ▓██▄   ▓██  ▀█ ██▒▒██▒▓██░ ██▓▒▒ ▓██░ ▒░",
                    "  ▒   ██▒▓██▒  ▐▌██▒░██░▒██▄█▓▒ ▒░ ▓██▓ ░ ",
                    "▒██████▒▒▒██░   ▓██░░██░▒██▒ ░  ░  ▒██▒ ░ ",
                ];

                let logo_text: Vec<Line> = logo
                    .iter()
                    .map(|line| {
                        Line::from(Span::styled(*line, Style::default().fg(Color::Magenta)))
                    })
                    .collect();

                let logo_widget = Paragraph::new(logo_text).alignment(Alignment::Center);
                f.render_widget(logo_widget, main_chunks[1]);

                // Draw daemon status
                let status_text = match state.daemon_status {
                    Some(pid) => {
                        vec![
                            Line::from(vec![
                                Span::styled("Status: ", Style::default().fg(Color::White)),
                                Span::styled("● ", Style::default().fg(Color::Green)),
                                Span::styled("RUNNING", Style::default().fg(Color::Green)),
                                Span::styled(
                                    format!(" (PID: {})", pid),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]),
                            Line::from(vec![
                                Span::raw("To "),
                                Span::styled("stop", Style::default().fg(Color::Red)),
                                Span::raw(" the daemon, run: "),
                                Span::styled("snipt stop", Style::default().fg(Color::Yellow)),
                            ]),
                            Line::from(vec![
                                Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
                                Span::raw("The daemon must be running for text expansion to work"),
                            ]),
                        ]
                    }
                    None => {
                        vec![
                            Line::from(vec![
                                Span::styled("Status: ", Style::default().fg(Color::White)),
                                Span::styled("● ", Style::default().fg(Color::Red)),
                                Span::styled("STOPPED", Style::default().fg(Color::Red)),
                            ]),
                            Line::from(vec![
                                Span::raw("To "),
                                Span::styled("start", Style::default().fg(Color::Green)),
                                Span::raw(" the daemon, run: "),
                                Span::styled("snipt start", Style::default().fg(Color::Yellow)),
                            ]),
                            Line::from(vec![
                                Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
                                Span::raw("The daemon must be running for text expansion to work"),
                            ]),
                        ]
                    }
                };

                let status = Paragraph::new(status_text)
                    .style(Style::default())
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Daemon Status "),
                    );
                f.render_widget(status, main_chunks[2]);

                // Action area with elegant minimalistic design
                let action_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Actions ")
                    .title_alignment(Alignment::Center);

                let action_area = action_block.inner(main_chunks[3]);
                f.render_widget(action_block, main_chunks[3]);

                // Create visually rich action buttons with distinctive styling
                let action_styles = vec![
                    (Color::Cyan, "📁", "Manage your snippet collection"),
                    (Color::Green, "✨", "Create a new text expansion snippet"),
                ];

                // More compact button design
                let button_height = 2; // Reduced from 3 to 2
                let actions_count = actions.len() as u16;

                // Calculate precise vertical positioning to eliminate gaps
                let total_height = action_area.height;
                let total_content_height = actions_count * button_height;

                // Distribute buttons evenly with no extra space
                let position_offset = if total_height > total_content_height {
                    (total_height - total_content_height) / 2
                } else {
                    0
                };

                for (i, &action) in actions.iter().enumerate() {
                    let is_selected = i == state.selected_action;
                    let (color, icon, description) = action_styles[i];
                    let i = i as u16;

                    // Precise button positioning
                    let button_y = action_area.y + position_offset + (i * button_height);

                    // Enhanced button styling
                    let button_style = Style::default()
                        .fg(if is_selected { color } else { Color::DarkGray })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        });

                    // Wider buttons for better aesthetics
                    let button_area = Rect {
                        x: action_area.x + 2, // Reduced left margin
                        y: button_y,
                        width: action_area.width.saturating_sub(4), // Wider buttons
                        height: button_height,
                    };

                    // Elegant border style with gradient effects for selected items
                    let button_block = Block::default()
                        .borders(if is_selected {
                            Borders::ALL
                        } else {
                            Borders::NONE
                        })
                        .border_style(Style::default().fg(color))
                        .style(Style::default().bg(if is_selected {
                            Color::DarkGray
                        } else {
                            Color::Reset
                        }));

                    f.render_widget(button_block, button_area);

                    // More compact button content layout
                    let inner_area = Rect {
                        x: button_area.x + 1,
                        y: button_area.y,
                        width: button_area.width.saturating_sub(2),
                        height: button_area.height,
                    };

                    // Combined button text and description for more compact display
                    let button_text = vec![Line::from(vec![
                        Span::styled(
                            format!("{} {} ", icon, action),
                            button_style.add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("- {}", description),
                            Style::default().fg(if is_selected {
                                Color::White
                            } else {
                                Color::DarkGray
                            }),
                        ),
                    ])];

                    let button_content = Paragraph::new(button_text);
                    f.render_widget(button_content, inner_area);

                    // Artistic selection indicator
                    if is_selected {
                        // Dynamic arrow indicator
                        let indicator = Paragraph::new("▶").style(Style::default().fg(color));
                        f.render_widget(
                            indicator,
                            Rect {
                                x: button_area.x - 2,
                                y: button_area.y,
                                width: 2,
                                height: 1,
                            },
                        );

                        // Add a subtle highlight line on the right side too for symmetry
                        let right_indicator = Paragraph::new("◀").style(Style::default().fg(color));
                        f.render_widget(
                            right_indicator,
                            Rect {
                                x: button_area.x + button_area.width,
                                y: button_area.y,
                                width: 2,
                                height: 1,
                            },
                        );
                    }
                }

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
                                // Add New Snippet - fully restored
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
