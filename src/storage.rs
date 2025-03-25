use crate::config::{ensure_config_dir, get_db_file_path};
use crate::error::{Result, ScribeError};
use crate::models::SnippetEntry;
use std::fs;

/// Load all snippets from the database
pub fn load_snippets() -> Result<Vec<SnippetEntry>> {
    let path = get_db_file_path();
    if !path.exists() {
        return Err(ScribeError::DatabaseNotFound(
            path.to_string_lossy().to_string(),
        ));
    }

    let content = fs::read_to_string(&path)?;

    // Handle empty database file
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    serde_json::from_str(&content).map_err(|e| e.into())
}

/// Save snippets to the database file
pub fn save_snippets(snippets: &[SnippetEntry]) -> Result<()> {
    let config_dir = ensure_config_dir()?;
    let db_path = config_dir.join("scribe.json");

    let serialized = serde_json::to_string_pretty(&snippets)?;
    fs::write(&db_path, serialized)?;

    Ok(())
}

/// Add a new snippet
pub fn add_snippet(shortcut: String, snippet: String) -> Result<()> {
    let mut snippets = match load_snippets() {
        Ok(s) => s,
        Err(ScribeError::DatabaseNotFound(_)) => vec![],
        Err(e) => return Err(e),
    };

    let entry = SnippetEntry::new(shortcut, snippet);
    snippets.push(entry);
    save_snippets(&snippets)
}

/// Delete a snippet by shortcut
pub fn delete_snippet(shortcut: &str) -> Result<()> {
    let mut snippets = load_snippets()?;
    snippets.retain(|entry| entry.shortcut != shortcut);
    save_snippets(&snippets)
}

/// Update an existing snippet
pub fn update_snippet(shortcut: &str, new_snippet: String) -> Result<()> {
    let mut snippets = load_snippets()?;
    let mut updated = false;

    for entry in &mut snippets {
        if entry.shortcut == shortcut {
            entry.update_snippet(new_snippet.clone());
            updated = true;
        }
    }

    if !updated {
        return Err(ScribeError::Other(format!(
            "Shortcut '{}' not found",
            shortcut
        )));
    }

    save_snippets(&snippets)
}

/// Find a snippet by shortcut
pub fn find_snippet<'a>(snippets: &'a [SnippetEntry], shortcut: &str) -> Option<&'a SnippetEntry> {
    snippets.iter().find(|entry| entry.shortcut == shortcut)
}
