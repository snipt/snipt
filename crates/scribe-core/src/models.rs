use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnippetEntry {
    pub shortcut: String,
    pub snippet: String,
    pub timestamp: String,
}

impl SnippetEntry {
    pub fn new(shortcut: String, snippet: String) -> Self {
        Self {
            shortcut,
            snippet,
            timestamp: Local::now().to_rfc3339(),
        }
    }

    pub fn update_snippet(&mut self, new_snippet: String) {
        self.snippet = new_snippet;
        self.timestamp = Local::now().to_rfc3339();
    }

    pub fn formatted_time(&self) -> String {
        let entry_time = DateTime::parse_from_rfc3339(&self.timestamp)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|_| Local::now());

        let now = Local::now();
        let duration = now.signed_duration_since(entry_time);

        if duration.num_seconds() < 60 {
            format!("{}s ago", duration.num_seconds())
        } else if duration.num_minutes() < 60 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{}h ago", duration.num_hours())
        } else {
            format!("{}d ago", duration.num_days())
        }
    }
}
