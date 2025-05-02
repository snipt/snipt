use enigo::{Direction, Key, Keyboard};
use std::fmt;

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
    Text(String),                           // Expand as text
    Execute(String),                        // Execute as script/URL/command
    ExecuteWithParams(String, Vec<String>), // Execute with parameters
}

impl fmt::Display for ExpansionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = match self {
            ExpansionType::Text(content) => format!("{}{}", SPECIAL_CHAR, content),
            ExpansionType::Execute(content) => format!("{}{}", EXECUTE_CHAR, content),
            ExpansionType::ExecuteWithParams(content, params) => {
                let params_str = params.join(",");
                format!("{}{}({})", EXECUTE_CHAR, content, params_str)
            }
        };
        write!(f, "{}", formatted)
    }
}

impl ExpansionType {
    /// Get the content of the expansion type without the prefix character
    pub fn content(&self) -> &str {
        match self {
            ExpansionType::Text(content) => content,
            ExpansionType::Execute(content) => content,
            ExpansionType::ExecuteWithParams(content, _) => content,
        }
    }

    /// Get the parameters for execution, if any
    pub fn params(&self) -> Option<&Vec<String>> {
        match self {
            ExpansionType::ExecuteWithParams(_, params) => Some(params),
            _ => None,
        }
    }

    /// Determine if this is a text expansion
    pub fn is_text(&self) -> bool {
        matches!(self, ExpansionType::Text(_))
    }

    /// Determine if this is an execution expansion
    pub fn is_execute(&self) -> bool {
        matches!(
            self,
            ExpansionType::Execute(_) | ExpansionType::ExecuteWithParams(_, _)
        )
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

    // Look for exact matches first (original behavior)
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

    // Look for shortcuts with parameter placeholders like "sum(a,b)"
    if first_char == EXECUTE_CHAR {
        // Check for both parameterized shortcut definitions and actual values
        for entry in snippets {
            if let Some(base_shortcut) = extract_base_shortcut(&entry.shortcut) {
                // This is a shortcut with parameter syntax like "sum(a,b)"

                // Check if the current input starts with this base shortcut
                if shortcut.starts_with(base_shortcut)
                    && shortcut.contains('(')
                    && shortcut.ends_with(')')
                {
                    // Extract parameters from the user input
                    if let Some(params) = extract_params_from_input(shortcut) {
                        // Extract placeholders from the shortcut definition
                        let placeholders = extract_placeholders(&entry.shortcut);

                        // Create a mapping from placeholders to actual values
                        let param_map = create_param_mapping(&placeholders, &params);

                        // Apply parameter substitution to the snippet content
                        let modified_content = apply_param_mapping(&entry.snippet, &param_map);

                        return Ok(Some(ExpansionType::Execute(modified_content)));
                    }
                }
            }
        }
    }

    // No matching shortcut found
    Ok(None)
}

/// Extract the base shortcut from a parameterized shortcut like "sum(a,b)" -> "sum"
fn extract_base_shortcut(shortcut: &str) -> Option<&str> {
    if shortcut.contains('(') {
        let parts: Vec<&str> = shortcut.split('(').collect();
        if parts.len() >= 2 {
            return Some(parts[0]);
        }
    }
    None
}

/// Extract parameters from input like "sum(2,3)" -> ["2", "3"]
fn extract_params_from_input(input: &str) -> Option<Vec<String>> {
    if let Some(open_idx) = input.find('(') {
        if let Some(close_idx) = input.rfind(')') {
            if close_idx > open_idx {
                let params_str = &input[open_idx + 1..close_idx];
                let params: Vec<String> = params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                return Some(params);
            }
        }
    }
    None
}

/// Extract placeholders from shortcut like "sum(a,b)" -> ["a", "b"]
fn extract_placeholders(shortcut: &str) -> Vec<String> {
    if let Some(open_idx) = shortcut.find('(') {
        if let Some(close_idx) = shortcut.rfind(')') {
            if close_idx > open_idx {
                let params_str = &shortcut[open_idx + 1..close_idx];
                return params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Create a mapping from placeholders to actual values
fn create_param_mapping(
    placeholders: &[String],
    values: &[String],
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for (i, placeholder) in placeholders.iter().enumerate() {
        if i < values.len() {
            map.insert(placeholder.clone(), values[i].clone());
        }
    }

    map
}

/// Apply parameter mapping to snippet content
fn apply_param_mapping(
    content: &str,
    param_map: &std::collections::HashMap<String, String>,
) -> String {
    let mut result = content.to_string();

    for (placeholder, value) in param_map {
        // Replace ${placeholder} with value
        let placeholder_pattern = format!("${{{}}}", placeholder);
        result = result.replace(&placeholder_pattern, value);

        // Also replace $placeholder with value
        let simple_pattern = format!("${}", placeholder);
        result = result.replace(&simple_pattern, value);
    }

    result
}

/// Handle text expansion or script execution
pub fn handle_expansion(to_delete: usize, expansion_type: ExpansionType) -> Result<()> {
    match expansion_type {
        ExpansionType::Text(text) => {
            // This is the original text expansion behavior
            replace_text(to_delete, &text)
        }
        ExpansionType::Execute(content) => {
            // Execute the snippet content without parameters
            execute_snippet(to_delete, &content, None)
        }
        ExpansionType::ExecuteWithParams(content, params) => {
            // Execute the snippet content with parameters
            execute_snippet(to_delete, &content, Some(&params))
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

    Ok(())
}
