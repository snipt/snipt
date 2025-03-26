use enigo::{Direction, Key, Keyboard};

use crate::config::SPECIAL_CHAR;
use crate::error::Result;
use crate::keyboard::{create_keyboard_controller, send_backspace};
use crate::models::SnippetEntry;
use crate::ScribeError;
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

pub fn type_text_with_formatting(keyboard: &mut impl Keyboard, text: &str) -> Result<()> {
    // Split into lines and type each line with proper newlines
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            // Type a newline between lines (not before the first line)
            match keyboard.key(Key::Return, Direction::Click) {
                Ok(_) => {}
                Err(err) => {
                    return Err(ScribeError::Enigo(format!(
                        "Failed to type newline: {}",
                        err
                    )))
                }
            }

            // Small delay after newline to ensure it registers properly
            thread::sleep(Duration::from_millis(10));
        }

        // Type the line content
        if !line.is_empty() {
            match keyboard.text(line) {
                Ok(_) => {}
                Err(err) => {
                    return Err(ScribeError::Enigo(format!("Failed to type text: {}", err)))
                }
            }

            // Small delay after each line for reliability
            thread::sleep(Duration::from_millis(5));
        }
    }

    Ok(())
}

/// Replace text in the editor by sending keyboard events
pub fn replace_text(to_delete: usize, replacement: &str) -> Result<()> {
    let mut keyboard = create_keyboard_controller()?;

    // Delete the text (shortcut and the special character)
    send_backspace(&mut keyboard, to_delete)?;

    // Small delay before typing the replacement
    thread::sleep(Duration::from_millis(10));

    // Type the expanded text with formatting preserved
    type_text_with_formatting(&mut keyboard, replacement)?;

    Ok(())
}
