use rdev::{self, EventType, Key as RdevKey};
use snipt_core::clipboard::get_clipboard_text;
use snipt_core::config::{EXECUTE_CHAR, SPECIAL_CHAR};
use snipt_core::expansion::process_expansion;
use snipt_core::handle_expansion;
use snipt_core::keyboard::rdev_key_to_char;
use snipt_core::models::SnippetEntry;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Starts listening for keyboard events and handles text expansion
pub fn start_keyboard_listener(
    snippets: Arc<Mutex<Vec<SnippetEntry>>>,
    running: Arc<Mutex<bool>>,
) -> JoinHandle<()> {
    // Buffer for text accumulation with a timestamp for each character
    let text_buffer = Arc::new(Mutex::new(Vec::<(char, Instant)>::new()));
    let buffer_clone = Arc::clone(&text_buffer);

    // Flag to track if we've just performed an expansion
    let just_expanded = Arc::new(Mutex::new(false));
    let expanded_flag_clone = Arc::clone(&just_expanded);

    // Track modifier key states
    let cmd_pressed = Arc::new(Mutex::new(false));
    let ctrl_pressed = Arc::new(Mutex::new(false));
    let cmd_clone = Arc::clone(&cmd_pressed);
    let ctrl_clone = Arc::clone(&ctrl_pressed);

    // Clone for the thread
    let snippets_clone = Arc::clone(&snippets);
    let running_clone = Arc::clone(&running);

    thread::spawn(move || {
        // Create a callback function closure
        let callback = move |event: rdev::Event| {
            if !*running_clone.lock().unwrap() {
                return;
            }

            // Update modifier key states
            match event.event_type {
                EventType::KeyPress(key) => match key {
                    RdevKey::MetaLeft | RdevKey::MetaRight => {
                        *cmd_clone.lock().unwrap() = true;
                    }
                    RdevKey::ControlLeft | RdevKey::ControlRight => {
                        *ctrl_clone.lock().unwrap() = true;
                    }
                    _ => {}
                },
                EventType::KeyRelease(key) => match key {
                    RdevKey::MetaLeft | RdevKey::MetaRight => {
                        *cmd_clone.lock().unwrap() = false;
                    }
                    RdevKey::ControlLeft | RdevKey::ControlRight => {
                        *ctrl_clone.lock().unwrap() = false;
                    }
                    _ => {}
                },
                _ => {} // Handle all other event types
            }
            if let EventType::KeyPress(key) = event.event_type {
                let mut buffer = buffer_clone.lock().unwrap();
                let mut just_expanded = expanded_flag_clone.lock().unwrap();

                // Handle paste command (Cmd+V on macOS, Ctrl+V on other platforms)
                let is_paste = match key {
                    RdevKey::KeyV => {
                        #[cfg(target_os = "macos")]
                        {
                            *cmd_clone.lock().unwrap()
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            *ctrl_clone.lock().unwrap()
                        }
                    }
                    _ => false,
                };

                if is_paste {
                    // Try to get clipboard content
                    if let Ok(clipboard_text) = get_clipboard_text() {
                        // Add all characters from the clipboard to the buffer at once
                        for c in clipboard_text.chars() {
                            buffer.push((c, Instant::now()));
                        }

                        // Check for expansion only once after the entire paste
                        let buffer_text: String = buffer.iter().map(|(c, _)| *c).collect();
                        let snippets_guard = snippets_clone.lock().unwrap();

                        if let Ok(Some(expansion)) =
                            process_expansion(&buffer_text, &snippets_guard)
                        {
                            // Found a matching expansion pattern!
                            let chars_to_delete = buffer_text.len();

                            // Handle the expansion or execution
                            let _ = handle_expansion(chars_to_delete, expansion);

                            // Set flag that we just expanded/executed
                            *just_expanded = true;

                            // Clear buffer completely
                            buffer.clear();
                            return;
                        }

                        // If we didn't expand, we should keep the buffer for potential future matches
                        if !*just_expanded {
                            return;
                        }
                    }
                }

                // Handle special keys
                match key {
                    RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                        // Check if we should expand the current buffer
                        if !buffer.is_empty() {
                            // Get the characters from the buffer, ignore timestamps
                            let buffer_text: String = buffer.iter().map(|(c, _)| *c).collect();

                            // Check if we're in the middle of a function-call syntax with open parenthesis
                            if buffer_text.contains('(') && !buffer_text.contains(')') {
                                // Still collecting parameters, don't expand yet
                                buffer.push((' ', Instant::now()));
                                return;
                            }

                            let snippets_guard = snippets_clone.lock().unwrap();
                            if let Ok(Some(expansion)) =
                                process_expansion(&buffer_text, &snippets_guard)
                            {
                                // Delete the special character and shortcut, then expand or execute
                                let _ = handle_expansion(buffer_text.len(), expansion);

                                // Set flag that we just expanded
                                *just_expanded = true;

                                // Clear buffer completely for fresh start
                                buffer.clear();
                                return; // Skip adding the space/newline/tab
                            }
                        }

                        // If we didn't expand or buffer was empty, add the space/newline/tab
                        // But first check if we need to reset after expansion
                        if *just_expanded {
                            buffer.clear();
                            *just_expanded = false;
                        }

                        // Let the key through (don't add to our buffer though)
                        buffer.clear();
                    }
                    RdevKey::Backspace => {
                        if !buffer.is_empty() {
                            buffer.pop();
                        }
                    }
                    RdevKey::Escape => {
                        // Clear buffer on escape
                        buffer.clear();
                    }
                    _ => {
                        // If we just expanded and now typing a new character,
                        // reset buffer to start fresh
                        if *just_expanded {
                            buffer.clear();
                            *just_expanded = false;
                        }

                        // Add the character to our buffer
                        if let Some(c) = rdev_key_to_char(&key, &event) {
                            buffer.push((c, Instant::now()));

                            // Get a lock on snippets for the checks
                            let snippets_guard = snippets_clone.lock().unwrap();

                            // Check if this is a closing parenthesis and we might have a complete function call
                            if c == ')' {
                                let buffer_text: String = buffer.iter().map(|(c, _)| *c).collect();

                                // Check for function call pattern
                                if buffer_text.starts_with(EXECUTE_CHAR)
                                    && buffer_text.contains('(')
                                {
                                    if let Ok(Some(expansion)) =
                                        process_expansion(&buffer_text, &snippets_guard)
                                    {
                                        // Found a matching function call pattern!
                                        let chars_to_delete = buffer_text.len();

                                        // Handle the expansion with parameters
                                        let _ = handle_expansion(chars_to_delete, expansion);

                                        // Set flag that we just expanded/executed
                                        *just_expanded = true;

                                        // Clear buffer completely
                                        buffer.clear();
                                        return;
                                    }
                                }
                            }

                            // The most recent character could be a trigger
                            if (c == SPECIAL_CHAR || c == EXECUTE_CHAR) && buffer.len() == 1 {
                                // Just added a potential trigger, continue collecting
                                drop(snippets_guard);
                                return;
                            }

                            // Look for both text expansion and execution patterns in the buffer
                            // This handles triggers in the middle of text
                            for i in 0..buffer.len() {
                                let first_char = buffer[i].0;
                                if (first_char == SPECIAL_CHAR || first_char == EXECUTE_CHAR)
                                    && i < buffer.len() - 1
                                {
                                    // Extract potential snippet from this position onward
                                    let potential_snippet: String =
                                        buffer[i..].iter().map(|(c, _)| *c).collect();

                                    // Skip if we have an open parenthesis without a closing one
                                    // This allows function-call syntax to be collected fully
                                    if potential_snippet.contains('(')
                                        && !potential_snippet.contains(')')
                                    {
                                        continue;
                                    }

                                    if let Ok(Some(expansion)) =
                                        process_expansion(&potential_snippet, &snippets_guard)
                                    {
                                        // Found a matching expansion pattern!
                                        // Calculate how many characters to delete (the trigger and shortcut)
                                        let chars_to_delete = potential_snippet.len();

                                        // Handle the expansion or execution
                                        let _ = handle_expansion(chars_to_delete, expansion);

                                        // Set flag that we just expanded/executed
                                        *just_expanded = true;

                                        // Remove the expanded part from buffer
                                        buffer.drain(i..);
                                        return;
                                    }
                                }
                            }

                            // Clean up old characters (older than 10 seconds)
                            let now = Instant::now();
                            buffer.retain(|(_, timestamp)| {
                                now.duration_since(*timestamp) < Duration::from_secs(10)
                            });

                            // Cap buffer size to prevent memory bloat (100 chars is plenty)
                            while buffer.len() > 100 {
                                buffer.remove(0);
                            }
                        }
                    }
                }
            }
        };

        // Register the callback
        if let Err(error) = rdev::listen(callback) {
            eprintln!("Error: Unable to listen for keyboard events: {:?}", error);
        }
    })
}
