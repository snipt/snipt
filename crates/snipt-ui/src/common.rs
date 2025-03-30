use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};
use snipt_core::Result;
use std::{thread, time::Duration};

// Helper function to show messages in a popup
pub fn show_message<B: ratatui::backend::Backend>(
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
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
