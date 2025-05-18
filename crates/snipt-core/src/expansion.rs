use enigo::{Direction, Key, Keyboard};
use std::fmt;
use std::process::Command;

use crate::config::{EXECUTE_CHAR, SPECIAL_CHAR};
use crate::error::Result;
use crate::execution::execute_snippet;
use crate::keyboard::{create_keyboard_controller, send_backspace};
use crate::models::SnippetEntry;
use crate::SniptError;
use std::thread;
use std::time::Duration;

/// Represents the expansion style to apply based on the current application context
pub enum ExpansionStyle {
    /// Default expansion style - expand the snippet content directly
    Default,
    /// Hyperlink style - used for platforms like  Linear, Slack, Teams
    /// that support hyperlinks rather than direct content pasting
    Hyperlink,
}

/// Represents the type of expansion to perform
pub enum ExpansionType {
    Text(String, ExpansionStyle, String), // Expand as text with style and original shortcut
    Execute(String, ExpansionStyle, String), // Execute as script/URL/command with style and original shortcut
    ExecuteWithParams(String, Vec<String>, ExpansionStyle, String), // Execute with parameters with style and original shortcut
}

impl fmt::Display for ExpansionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = match self {
            ExpansionType::Text(content, _, _) => format!("{}{}", SPECIAL_CHAR, content),
            ExpansionType::Execute(content, _, _) => format!("{}{}", EXECUTE_CHAR, content),
            ExpansionType::ExecuteWithParams(content, params, _, _) => {
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
            ExpansionType::Text(content, _, _) => content,
            ExpansionType::Execute(content, _, _) => content,
            ExpansionType::ExecuteWithParams(content, _, _, _) => content,
        }
    }

    /// Get the parameters for execution, if any
    pub fn params(&self) -> Option<&Vec<String>> {
        match self {
            ExpansionType::ExecuteWithParams(_, params, _, _) => Some(params),
            _ => None,
        }
    }

    /// Get the expansion style
    pub fn style(&self) -> &ExpansionStyle {
        match self {
            ExpansionType::Text(_, style, _) => style,
            ExpansionType::Execute(_, style, _) => style,
            ExpansionType::ExecuteWithParams(_, _, style, _) => style,
        }
    }

    /// Get the original shortcut (for all variants)
    pub fn shortcut(&self) -> Option<&str> {
        match self {
            ExpansionType::Text(_, _, shortcut) => Some(shortcut),
            ExpansionType::Execute(_, _, shortcut) => Some(shortcut),
            ExpansionType::ExecuteWithParams(_, _, _, shortcut) => Some(shortcut),
        }
    }

    /// Determine if this is a text expansion
    pub fn is_text(&self) -> bool {
        matches!(self, ExpansionType::Text(_, _, _))
    }

    /// Determine if this is an execution expansion
    pub fn is_execute(&self) -> bool {
        matches!(
            self,
            ExpansionType::Execute(_, _, _) | ExpansionType::ExecuteWithParams(_, _, _, _)
        )
    }
}

/// Helper function to detect the frontmost application (macOS)
#[cfg(target_os = "macos")]
pub fn get_frontmost_app() -> String {
    // Try to get the frontmost app using AppleScript
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output();

    if let Ok(output) = output {
        let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return app_name;
    }

    String::new() // Return empty string if we couldn't determine the app
}

/// Helper function to detect the frontmost application (Linux)
#[cfg(target_os = "linux")]
pub fn get_frontmost_app() -> String {
    // Check if running under Wayland
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        // xdotool is unreliable on Wayland for getting the active Wayland window.
        // It might get an XWayland window or fail. Returning empty string for now.
        // More sophisticated Wayland-specific detection is complex and DE-dependent.
        return String::new();
    }

    // Try to get active window using xdotool (needs to be installed) - for X11
    let output = Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output();

    if let Ok(output) = output {
        let window_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return window_name;
    }

    String::new() // Return empty string if we couldn't determine
}

/// Helper function to detect the frontmost application (Windows)
#[cfg(target_os = "windows")]
pub fn get_frontmost_app() -> String {
    // Try to get active window using PowerShell
    let output = Command::new("powershell")
        .args(&[
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.ActiveForm]::ActiveForm.Text",
        ])
        .output();

    if let Ok(output) = output {
        let window_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return window_name;
    }

    String::new() // Return empty string if we couldn't determine
}

/// Determine the expansion style based on the current application
pub fn determine_expansion_style() -> ExpansionStyle {
    // Get the current app name and normalize to lowercase for case-insensitive matching
    let app_name = get_frontmost_app().to_lowercase();

    // List of applications that should use the Hyperlink expansion style
    // Note: This doesn't mean the app supports rich text hyperlinking - we determine
    // the specific format in format_app_specific_hyperlink based on each app's capabilities
    let hyperlink_apps = [
        "linear",
        "slack",
        "microsoft teams",
        "teams",
        "discord",
        "telegram",
        "chrome",
        "firefox",
        "safari",
        "edge",
        "brave",
        "opera",
    ];

    // Check if the current app is in the hyperlink apps list
    for app in hyperlink_apps.iter() {
        if app_name.contains(app) {
            return ExpansionStyle::Hyperlink;
        }
    }

    // Default to normal expansion
    ExpansionStyle::Default
}

/// Process text buffer to check for text expansion trigger
pub fn process_expansion(buffer: &str, snippets: &[SnippetEntry]) -> Result<Option<ExpansionType>> {
    // Check if the buffer is valid for expansion
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

    // Determine expansion style based on current application
    let expansion_style = determine_expansion_style();

    // Look for exact matches first (original behavior)
    for entry in snippets {
        if entry.shortcut == shortcut {
            return if first_char == SPECIAL_CHAR {
                // Expansion trigger
                Ok(Some(ExpansionType::Text(
                    entry.snippet.clone(),
                    expansion_style,
                    shortcut.to_string(),
                )))
            } else if first_char == EXECUTE_CHAR {
                // Execution trigger
                Ok(Some(ExpansionType::Execute(
                    entry.snippet.clone(),
                    expansion_style,
                    shortcut.to_string(),
                )))
            } else {
                // Unreachable based on our earlier filter, but here for safety
                Ok(None)
            };
        }
    }

    // Only check for parameterized snippets if relevant
    if first_char == EXECUTE_CHAR && shortcut.contains('(') && shortcut.ends_with(')') {
        // Extract the base shortcut from the input (without parameters)
        if let Some(input_base) = extract_base_shortcut(shortcut) {
            // Look for matching base shortcuts
            for entry in snippets {
                // Check if the snippet entry has parameters (contains '(' and ')')
                if entry.shortcut.contains('(') && entry.shortcut.contains(')') {
                    if let Some(entry_base) = extract_base_shortcut(&entry.shortcut) {
                        // Compare the base parts (without parameters)
                        if input_base == entry_base {
                            // Extract parameters from the user input
                            if let Some(params) = extract_params_from_input(shortcut) {
                                // Extract placeholders from the shortcut definition
                                let placeholders = extract_placeholders(&entry.shortcut);

                                // Create a mapping from placeholders to actual values
                                let param_map = create_param_mapping(&placeholders, &params);

                                // Apply parameter substitution to the snippet content
                                let modified_content =
                                    apply_param_mapping(&entry.snippet, &param_map);

                                return Ok(Some(ExpansionType::ExecuteWithParams(
                                    modified_content,
                                    params,
                                    expansion_style,
                                    entry_base.to_string(),
                                )));
                            }
                        }
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
                // Handle empty params case
                if params_str.trim().is_empty() {
                    return Some(Vec::new());
                }

                let params: Vec<String> = params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()) // Filter out empty strings
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
    // Early return if there are no parameters or the content is empty
    if param_map.is_empty() || content.is_empty() {
        return content.to_string();
    }

    let mut result = content.to_string();

    // Apply all replacements in a single pass
    for (placeholder, value) in param_map {
        // Replace ${placeholder} with value
        let placeholder_pattern = format!("${{{}}}", placeholder);
        result = result.replace(&placeholder_pattern, value);

        // Also replace $placeholder with value
        let simple_pattern = format!("${}", placeholder);
        result = result.replace(&simple_pattern, value);

        // Also replace positional placeholders if placeholder is numeric
        if let Ok(pos) = placeholder.parse::<usize>() {
            // Replace $1, $2, etc.
            let pos_pattern = format!("${}", pos);
            result = result.replace(&pos_pattern, value);

            // Replace ${1}, ${2}, etc.
            let pos_brace_pattern = format!("${{{}}}", pos);
            result = result.replace(&pos_brace_pattern, value);
        }
    }

    // Handle $* (all parameters) if present in content
    if result.contains("$*") || result.contains("${*}") {
        let all_values = param_map
            .values()
            .cloned()
            .collect::<Vec<String>>()
            .join(" ");
        result = result.replace("$*", &all_values);
        result = result.replace("${*}", &all_values);
    }

    result
}

/// Handle text expansion or script execution based on the expansion style
pub fn handle_expansion(to_delete: usize, expansion_type: ExpansionType) -> Result<()> {
    match expansion_type {
        ExpansionType::Text(text, style, shortcut) => {
            match style {
                ExpansionStyle::Default => {
                    // Original text expansion behavior
                    replace_text(to_delete, &text)
                }
                ExpansionStyle::Hyperlink => {
                    // For platforms that support hyperlinks, transform to a hyperlink
                    // Check if the text looks like a URL
                    if text.starts_with("http://")
                        || text.starts_with("https://")
                        || text.starts_with("www.")
                    {
                        // Get the app name for platform-specific formatting
                        let app_name = get_frontmost_app();

                        // Use the original shortcut as the display text
                        let hyperlink = format_app_specific_hyperlink(&app_name, &shortcut, &text);
                        replace_text(to_delete, &hyperlink)
                    } else {
                        // For non-URLs, just use the original expansion
                        replace_text(to_delete, &text)
                    }
                }
            }
        }
        ExpansionType::Execute(content, style, shortcut) => {
            match style {
                ExpansionStyle::Default => {
                    // Original execution behavior
                    execute_snippet(to_delete, &content, None)
                }
                ExpansionStyle::Hyperlink => {
                    // For URLs specifically, we can format as a hyperlink
                    if content.starts_with("http://")
                        || content.starts_with("https://")
                        || content.starts_with("www.")
                    {
                        // Get the app name for platform-specific formatting
                        let app_name = get_frontmost_app();

                        // Format hyperlink based on the app
                        let hyperlink =
                            format_app_specific_hyperlink(&app_name, &shortcut, &content);
                        replace_text(to_delete, &hyperlink)
                    } else {
                        // Fall back to default behavior for non-URLs
                        execute_snippet(to_delete, &content, None)
                    }
                }
            }
        }
        ExpansionType::ExecuteWithParams(content, params, style, shortcut) => {
            match style {
                ExpansionStyle::Default => {
                    // Original parameterized execution behavior
                    execute_snippet(to_delete, &content, Some(&params))
                }
                ExpansionStyle::Hyperlink => {
                    // Similar handling as Execute
                    if content.starts_with("http://")
                        || content.starts_with("https://")
                        || content.starts_with("www.")
                    {
                        // Get the app name for platform-specific formatting
                        let app_name = get_frontmost_app();

                        // Format hyperlink based on the app
                        let hyperlink =
                            format_app_specific_hyperlink(&app_name, &shortcut, &content);
                        replace_text(to_delete, &hyperlink)
                    } else {
                        // Fall back to default behavior for non-URLs
                        execute_snippet(to_delete, &content, Some(&params))
                    }
                }
            }
        }
    }
}

/// Format a hyperlink based on the specific application's native link format
fn format_app_specific_hyperlink(app_name: &str, display_text: &str, url: &str) -> String {
    // Normalize the app name to lowercase for case-insensitive matching
    let app_name = app_name.to_lowercase();

    // Different apps use different hyperlink formats based on their native link creation methods
    if app_name.contains("teams")
        || app_name.contains("microsoft")
        || app_name.contains("discord")
        || app_name.contains("linear")
    {
        // These apps all use markdown format: [display text](url)
        format!("[{}]({})", display_text, url)
    } else if app_name.contains("outlook") || app_name.contains("mail") {
        // Outlook and most email clients support HTML links
        format!("<a href=\"{}\">{}</a>", url, display_text)
    } else {
        // Default to raw URL for all other apps (slack, telegram, browsers, etc.)
        // This is most compatible and often fastest option
        url.to_string()
    }
}

pub fn type_text_with_formatting(keyboard: &mut impl Keyboard, text: &str) -> Result<()> {
    // Set a reasonable chunk size to avoid overwhelming the keyboard buffer
    // Increased chunk size for better performance
    const CHUNK_SIZE: usize = 1024;

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

            // Reduced delay after newline
            thread::sleep(Duration::from_millis(5));
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

                // Reduced delay between chunks
                thread::sleep(Duration::from_millis(5));
            }
        } else if !line.is_empty() {
            // Type the line content directly if it's short
            match keyboard.text(line) {
                Ok(_) => {}
                Err(err) => return Err(SniptError::Enigo(format!("Failed to type text: {}", err))),
            }
        }
        // Reduced delay after each line for reliability
        thread::sleep(Duration::from_millis(2));
    }

    Ok(())
}

/// Replace text in the editor by sending keyboard events
pub fn replace_text(to_delete: usize, replacement: &str) -> Result<()> {
    let mut keyboard = create_keyboard_controller()?;

    // Delete the text (shortcut and the special character)
    send_backspace(&mut keyboard, to_delete)?;

    // Minimal delay before typing the replacement (reduced from 10ms)
    thread::sleep(Duration::from_millis(3));

    // Type the expanded text with formatting preserved
    type_text_with_formatting(&mut keyboard, replacement)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SnippetEntry;

    #[test]
    fn test_determine_expansion_style() {
        // Override get_frontmost_app for testing
        assert!(matches!(
            determine_expansion_style_with_app("linear"),
            ExpansionStyle::Hyperlink
        ));
        assert!(matches!(
            determine_expansion_style_with_app("slack"),
            ExpansionStyle::Hyperlink
        ));
        assert!(matches!(
            determine_expansion_style_with_app("Microsoft Teams"),
            ExpansionStyle::Hyperlink
        ));
        assert!(matches!(
            determine_expansion_style_with_app("Google Chrome"),
            ExpansionStyle::Hyperlink
        ));
        assert!(matches!(
            determine_expansion_style_with_app("Firefox"),
            ExpansionStyle::Hyperlink
        ));
        assert!(matches!(
            determine_expansion_style_with_app("Safari"),
            ExpansionStyle::Hyperlink
        ));

        // Test with non-hyperlink apps
        assert!(matches!(
            determine_expansion_style_with_app("terminal"),
            ExpansionStyle::Default
        ));
        assert!(matches!(
            determine_expansion_style_with_app("vscode"),
            ExpansionStyle::Default
        ));
    }

    // Test helper that allows us to inject a specific app name
    fn determine_expansion_style_with_app(app_name: &str) -> ExpansionStyle {
        // List of applications that should use the Hyperlink expansion style
        // Note: This doesn't mean the app supports rich text hyperlinking - we determine
        // the specific format in format_app_specific_hyperlink based on each app's capabilities
        let hyperlink_apps = [
            "linear",
            "slack",
            "microsoft teams",
            "teams",
            "discord",
            "telegram",
            "chrome",
            "firefox",
            "safari",
            "edge",
            "brave",
            "opera",
        ];

        // Check if the current app is in the hyperlink apps list - normalize to lowercase
        let app_name = app_name.to_lowercase();

        for app in hyperlink_apps.iter() {
            if app_name.contains(app) {
                return ExpansionStyle::Hyperlink;
            }
        }

        // Default to normal expansion
        ExpansionStyle::Default
    }

    #[test]
    fn test_hyperlink_expansion() {
        let snippets = vec![
            SnippetEntry {
                shortcut: "hello".to_string(),
                snippet: "Hello, world!".to_string(),
                timestamp: "2023-01-01T00:00:00+00:00".to_string(),
            },
            SnippetEntry {
                shortcut: "link".to_string(),
                snippet: "https://example.com".to_string(),
                timestamp: "2023-01-01T00:00:00+00:00".to_string(),
            },
        ];

        // In our tests, use the actual SPECIAL_CHAR constant from config
        let buffer_special = format!("{}hello", SPECIAL_CHAR);

        // Test normal expansion in default apps
        let result =
            process_expansion_with_style(&buffer_special, &snippets, ExpansionStyle::Default)
                .unwrap();
        assert!(result.is_some());
        let expansion = result.unwrap();
        assert!(matches!(
            expansion,
            ExpansionType::Text(_, ExpansionStyle::Default, _)
        ));
        assert_eq!(expansion.content(), "Hello, world!");

        // Test hyperlink expansion in specific apps
        let result =
            process_expansion_with_style(&buffer_special, &snippets, ExpansionStyle::Hyperlink)
                .unwrap();
        assert!(result.is_some());
        let expansion = result.unwrap();
        assert!(matches!(
            expansion,
            ExpansionType::Text(_, ExpansionStyle::Hyperlink, _)
        ));
        assert_eq!(expansion.content(), "Hello, world!");

        // Also test URL expansion
        let buffer_url = format!("{}link", EXECUTE_CHAR);
        let result =
            process_expansion_with_style(&buffer_url, &snippets, ExpansionStyle::Hyperlink)
                .unwrap();
        assert!(result.is_some());
        let expansion = result.unwrap();
        assert!(matches!(
            expansion,
            ExpansionType::Execute(_, ExpansionStyle::Hyperlink, _)
        ));
        assert_eq!(expansion.content(), "https://example.com");
        assert_eq!(expansion.shortcut().unwrap(), "link");
    }

    // Test helper that allows us to inject a specific expansion style
    fn process_expansion_with_style(
        buffer: &str,
        snippets: &[SnippetEntry],
        style: ExpansionStyle,
    ) -> Result<Option<ExpansionType>> {
        // This is similar to the real process_expansion but allows us to
        // specify the expansion style directly for testing

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

        // Use the provided expansion style instead of determining it
        let expansion_style = style;

        // Extract the shortcut without the special character
        let shortcut = &buffer[1..];

        // Look for exact matches
        for entry in snippets {
            if entry.shortcut == shortcut {
                if first_char == SPECIAL_CHAR {
                    // Expansion trigger
                    return Ok(Some(ExpansionType::Text(
                        entry.snippet.clone(),
                        expansion_style,
                        shortcut.to_string(),
                    )));
                } else if first_char == EXECUTE_CHAR {
                    // Execution trigger
                    return Ok(Some(ExpansionType::Execute(
                        entry.snippet.clone(),
                        expansion_style,
                        shortcut.to_string(),
                    )));
                }
            }
        }

        // No matching shortcut found
        Ok(None)
    }

    #[test]
    fn test_hyperlink_formatting() {
        // Test app-specific hyperlink formats with different casing
        // Slack doesn't support rich text hyperlinking - returns raw URL
        assert_eq!(
            format_app_specific_hyperlink("SlAcK", "google", "https://example.com"),
            "https://example.com"
        );

        // Discord uses markdown format [text](url)
        assert_eq!(
            format_app_specific_hyperlink("DisCoRd", "google", "https://example.com"),
            "[google](https://example.com)"
        );

        // Test case for Microsoft Teams with mixed casing - uses markdown format
        assert_eq!(
            format_app_specific_hyperlink("MicroSoft TEAMS", "google", "https://example.com"),
            "[google](https://example.com)"
        );

        // Test case for Linear - uses markdown format
        assert_eq!(
            format_app_specific_hyperlink("Linear", "google", "https://example.com"),
            "[google](https://example.com)"
        );

        // Test case for Telegram - returns raw URL due to inconsistent client support
        assert_eq!(
            format_app_specific_hyperlink("Telegram", "google", "https://example.com"),
            "https://example.com"
        );

        // Test case for browsers - should return raw URL
        assert_eq!(
            format_app_specific_hyperlink("Google Chrome", "google", "https://example.com"),
            "https://example.com"
        );

        // Test case for Safari - should return raw URL
        assert_eq!(
            format_app_specific_hyperlink("Safari", "google", "https://example.com"),
            "https://example.com"
        );

        // Test case for email clients - uses HTML format
        assert_eq!(
            format_app_specific_hyperlink("Outlook", "google", "https://example.com"),
            "<a href=\"https://example.com\">google</a>"
        );

        // Test default case for unknown apps - should return raw URL
        assert_eq!(
            format_app_specific_hyperlink("Unknown App", "google", "https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_parameterized_shortcuts() {
        let snippets = vec![
            SnippetEntry {
                shortcut: "sum(a,b)".to_string(),
                snippet: "The sum of $a and $b is ${a+b}".to_string(),
                timestamp: "2023-01-01T00:00:00+00:00".to_string(),
            },
            SnippetEntry {
                shortcut: "greet(name)".to_string(),
                snippet: "Hello, $name!".to_string(),
                timestamp: "2023-01-01T00:00:00+00:00".to_string(),
            },
        ];

        // Test parameterized expansion with sum
        let buffer_sum = format!("{}sum(10,20)", EXECUTE_CHAR);
        let result = process_expansion(&buffer_sum, &snippets).unwrap();
        assert!(result.is_some());
        let expansion = result.unwrap();
        assert!(matches!(
            expansion,
            ExpansionType::ExecuteWithParams(_, _, _, _)
        ));

        if let ExpansionType::ExecuteWithParams(content, params, _, shortcut) = expansion {
            assert_eq!(content, "The sum of 10 and 20 is ${a+b}");
            assert_eq!(params, vec!["10".to_string(), "20".to_string()]);
            assert_eq!(shortcut, "sum");
        }

        // Test parameterized expansion with greet
        let buffer_greet = format!("{}greet(World)", EXECUTE_CHAR);
        let result = process_expansion(&buffer_greet, &snippets).unwrap();
        assert!(result.is_some());
        let expansion = result.unwrap();
        assert!(matches!(
            expansion,
            ExpansionType::ExecuteWithParams(_, _, _, _)
        ));

        if let ExpansionType::ExecuteWithParams(content, params, _, shortcut) = expansion {
            assert_eq!(content, "Hello, World!");
            assert_eq!(params, vec!["World".to_string()]);
            assert_eq!(shortcut, "greet");
        }
    }

    #[test]
    fn test_parameter_mapping() {
        // Test basic parameter mapping
        let mut map = std::collections::HashMap::new();
        map.insert("name".to_string(), "John".to_string());
        map.insert("age".to_string(), "30".to_string());

        let content = "Name: $name, Age: $age, Name with braces: ${name}";
        let result = apply_param_mapping(content, &map);
        assert_eq!(result, "Name: John, Age: 30, Name with braces: John");

        // Test positional parameters
        let mut map = std::collections::HashMap::new();
        map.insert("1".to_string(), "First".to_string());
        map.insert("2".to_string(), "Second".to_string());

        let content = "First: $1, Second: $2, First with braces: ${1}";
        let result = apply_param_mapping(content, &map);
        assert_eq!(
            result,
            "First: First, Second: Second, First with braces: First"
        );

        // Test all parameters wildcard
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), "one".to_string());
        map.insert("b".to_string(), "two".to_string());
        map.insert("c".to_string(), "three".to_string());

        let content = "All params: $*, All params with braces: ${*}";
        let result = apply_param_mapping(content, &map);

        // Note: order might vary due to HashMap, so we check parts
        assert!(result.contains("one"));
        assert!(result.contains("two"));
        assert!(result.contains("three"));
        assert!(result.starts_with("All params: "));
        assert!(result.contains("All params with braces: "));
    }
}
