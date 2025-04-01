use enigo::{Direction, Key, Keyboard};

use crate::config::{EXECUTE_CHAR, SPECIAL_CHAR};
use crate::error::Result;
use crate::execution::execute_snippet;
use crate::keyboard::{create_keyboard_controller, send_backspace};
use crate::models::SnippetEntry;
use crate::SniptError;
use std::thread;
use std::time::Duration;

/// Represents the type of expansion to perform
pub enum ExpansionType {
    Text(String),    // Expand as text
    Execute(String), // Execute as script/URL/command
}

impl ExpansionType {
    /// Convert an ExpansionType to its string representation
    pub fn to_string(&self) -> String {
        match self {
            ExpansionType::Text(content) => format!("{}{}", SPECIAL_CHAR, content),
            ExpansionType::Execute(content) => format!("{}{}", EXECUTE_CHAR, content),
        }
    }

    /// Get the content of the expansion type without the prefix character
    pub fn content(&self) -> &str {
        match self {
            ExpansionType::Text(content) => content,
            ExpansionType::Execute(content) => content,
        }
    }

    /// Determine if this is a text expansion
    pub fn is_text(&self) -> bool {
        matches!(self, ExpansionType::Text(_))
    }

    /// Determine if this is an execution expansion
    pub fn is_execute(&self) -> bool {
        matches!(self, ExpansionType::Execute(_))
    }
}

/// Process text buffer to check for text expansion trigger
pub fn process_expansion(buffer: &str, snippets: &[SnippetEntry]) -> Result<Option<ExpansionType>> {
    // Check if the buffer starts with the special character
    if buffer.is_empty() {
        return Ok(None);
    }

    let first_char = buffer.chars().next().unwrap();

    if first_char != SPECIAL_CHAR && first_char != EXECUTE_CHAR {
        return Ok(None);
    }

    if buffer.len() <= 1 {
        return Ok(None);
    }

    // Extract the shortcut without the special character
    let shortcut = &buffer[1..];

    // Look for matching snippet
    for entry in snippets {
        if entry.shortcut == shortcut {
            if first_char == SPECIAL_CHAR {
                // Expansion trigger
                return Ok(Some(ExpansionType::Text(entry.snippet.clone())));
            } else if first_char == EXECUTE_CHAR {
                // Execution trigger
                return Ok(Some(ExpansionType::Execute(entry.snippet.clone())));
            }
        }
    }

    Ok(None)
}

/// Handle text expansion or script execution
pub fn handle_expansion(to_delete: usize, expansion_type: ExpansionType) -> Result<()> {
    match expansion_type {
        ExpansionType::Text(text) => {
            // This is the original text expansion behavior
            replace_text(to_delete + 1, &text)
        }
        ExpansionType::Execute(content) => {
            // Execute the snippet content
            execute_snippet(to_delete + 1, &content)
        }
    }
}

pub fn type_text_with_formatting(keyboard: &mut impl Keyboard, text: &str) -> Result<()> {
    // Set a reasonable chunk size to avoid overwhelming the keyboard buffer
    const CHUNK_SIZE: usize = 512;

    // Split into lines and type each line with proper newlines
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            // Type a newline between lines (not before the first line)
            match keyboard.key(Key::Return, Direction::Click) {
                Ok(_) => {}
                Err(err) => {
                    return Err(SniptError::Enigo(format!(
                        "Failed to type newline: {}",
                        err
                    )))
                }
            }

            // Small delay after newline to ensure it registers properly
            thread::sleep(Duration::from_millis(15));
        }

        // If line is very long, split it into manageable chunks
        if line.len() > CHUNK_SIZE {
            for chunk in line.chars().collect::<Vec<_>>().chunks(CHUNK_SIZE) {
                let chunk_str: String = chunk.iter().collect();
                match keyboard.text(&chunk_str) {
                    Ok(_) => {}
                    Err(err) => {
                        return Err(SniptError::Enigo(format!("Failed to type text: {}", err)))
                    }
                }

                // Small delay between chunks
                thread::sleep(Duration::from_millis(20));
            }
        } else if !line.is_empty() {
            // Type the line content directly if it's short
            match keyboard.text(line) {
                Ok(_) => {}
                Err(err) => return Err(SniptError::Enigo(format!("Failed to type text: {}", err))),
            }
        }
        // Small delay after each line for reliability
        thread::sleep(Duration::from_millis(10));
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
    // execute_snippet(&replacement)?;  // Remove or comment this line

    Ok(())
}
