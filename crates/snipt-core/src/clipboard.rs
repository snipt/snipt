use crate::error::{Result, SniptError};
use arboard::Clipboard;

/// Get the current clipboard content as text
pub fn get_clipboard_text() -> Result<String> {
    let mut clipboard = Clipboard::new().map_err(|e| SniptError::Clipboard(e.to_string()))?;
    clipboard
        .get_text()
        .map_err(|e| SniptError::Clipboard(e.to_string()))
}

/// Set the clipboard content as text
pub fn set_clipboard_text(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().map_err(|e| SniptError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text)
        .map_err(|e| SniptError::Clipboard(e.to_string()))
}

/// Check if the clipboard contains text
pub fn has_clipboard_text() -> bool {
    if let Ok(mut clipboard) = Clipboard::new() {
        clipboard.get_text().is_ok()
    } else {
        false
    }
}
