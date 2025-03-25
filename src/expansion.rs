use crate::config::SPECIAL_CHAR;
use crate::error::Result;
use crate::keyboard::{create_keyboard_controller, send_backspace, type_text};
use crate::models::SnippetEntry;
use std::thread;
use std::time::Duration;

/// Process text buffer to check for text expansion trigger
pub fn process_expansion(buffer: &str, snippets: &[SnippetEntry]) -> Result<Option<String>> {
    // Check if the buffer starts with the special character
    if !buffer.starts_with(SPECIAL_CHAR) || buffer.len() <= 1 {
        return Ok(None);
    }

    // Extract the shortcut without the special character
    let shortcut = &buffer[1..];

    // Look for matching snippet
    for entry in snippets {
        if entry.shortcut == shortcut {
            return Ok(Some(entry.snippet.clone()));
        }
    }

    Ok(None)
}

/// Replace text in the editor by sending keyboard events
pub fn replace_text(to_delete: usize, replacement: &str) -> Result<()> {
    let mut keyboard = create_keyboard_controller()?;

    // Delete the text (shortcut and the special character)
    send_backspace(&mut keyboard, to_delete)?;

    // Small delay before typing the replacement
    thread::sleep(Duration::from_millis(10));

    // Type the expanded text
    type_text(&mut keyboard, replacement)?;

    Ok(())
}
