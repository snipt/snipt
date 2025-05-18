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
        let callback = move |event: rdev::Event| -> Option<rdev::Event> {
            if !*running_clone.lock().unwrap() {
                return Some(event);
            }

            // Handle modifier state updates first, then decide if it's a KeyPress to process further
            match event.event_type {
                EventType::KeyPress(key) => {
                    // Update modifier state for KeyPress
                    match key {
                        RdevKey::MetaLeft | RdevKey::MetaRight => {
                            *cmd_clone.lock().unwrap() = true;
                        }
                        RdevKey::ControlLeft | RdevKey::ControlRight => {
                            *ctrl_clone.lock().unwrap() = true;
                        }
                        _ => {}
                    }
                    // Proceed to main KeyPress logic
                }
                EventType::KeyRelease(key) => {
                    match key {
                        RdevKey::MetaLeft | RdevKey::MetaRight => {
                            *cmd_clone.lock().unwrap() = false;
                        }
                        RdevKey::ControlLeft | RdevKey::ControlRight => {
                            *ctrl_clone.lock().unwrap() = false;
                        }
                        _ => {}
                    }
                    return Some(event);
                }
                _ => return Some(event),
            }

            // This point is reached only for EventType::KeyPress
            // We need to get the key again from event.event_type
            let key = match event.event_type {
                EventType::KeyPress(k) => k,
                _ => return Some(event),
            };

            let mut buffer = buffer_clone.lock().unwrap();
            let mut just_expanded_val = expanded_flag_clone.lock().unwrap();

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
                if let Ok(clipboard_text) = get_clipboard_text() {
                    let temp_buffer_for_paste_check: String =
                        buffer.iter().map(|(c, _)| *c).collect::<String>() + &clipboard_text;
                    let snippets_guard = snippets_clone.lock().unwrap();

                    if let Ok(Some(_)) =
                        process_expansion(&temp_buffer_for_paste_check, &snippets_guard)
                    {
                        let current_buffer_text_for_paste: String =
                            buffer.iter().map(|(c, _)| *c).collect();
                        let combined_text_for_check =
                            current_buffer_text_for_paste + &clipboard_text;

                        if let Ok(Some(expansion_from_paste)) =
                            process_expansion(&combined_text_for_check, &snippets_guard)
                        {
                            let _ = handle_expansion(
                                combined_text_for_check.len(),
                                expansion_from_paste,
                            );
                            *just_expanded_val = true;
                            buffer.clear();
                            return None;
                        }
                    }
                }
                return Some(event);
            }

            // Handle special keys
            match key {
                RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                    if !buffer.is_empty() {
                        let buffer_text: String = buffer.iter().map(|(c, _)| *c).collect();

                        if buffer_text.contains('(') && !buffer_text.contains(')') {
                            buffer.push((' ', Instant::now()));
                            return Some(event);
                        }

                        let snippets_guard = snippets_clone.lock().unwrap();
                        if let Ok(Some(expansion)) =
                            process_expansion(&buffer_text, &snippets_guard)
                        {
                            let _ = handle_expansion(buffer_text.len(), expansion);
                            *just_expanded_val = true;
                            buffer.clear();
                            return None;
                        }
                    }

                    if *just_expanded_val {
                        buffer.clear();
                        *just_expanded_val = false;
                    }
                    buffer.clear();
                    Some(event)
                }
                RdevKey::Backspace => {
                    if !buffer.is_empty() {
                        buffer.pop();
                    }
                    Some(event)
                }
                RdevKey::Escape => {
                    buffer.clear();
                    Some(event)
                }
                _ => {
                    if *just_expanded_val {
                        buffer.clear();
                        *just_expanded_val = false;
                    }

                    if let Some(c) = rdev_key_to_char(&key, &event) {
                        buffer.push((c, Instant::now()));

                        let snippets_guard = snippets_clone.lock().unwrap();

                        if c == ')' {
                            let buffer_text_fn: String = buffer.iter().map(|(c, _)| *c).collect();
                            if buffer_text_fn.starts_with(EXECUTE_CHAR)
                                && buffer_text_fn.contains('(')
                            {
                                if let Ok(Some(expansion)) =
                                    process_expansion(&buffer_text_fn, &snippets_guard)
                                {
                                    let _ = handle_expansion(buffer_text_fn.len(), expansion);
                                    *just_expanded_val = true;
                                    buffer.clear();
                                    return None;
                                }
                            }
                        }

                        if (c == SPECIAL_CHAR || c == EXECUTE_CHAR) && buffer.len() == 1 {
                            return Some(event);
                        }

                        for i in 0..buffer.len() {
                            let first_char = buffer[i].0;
                            if (first_char == SPECIAL_CHAR || first_char == EXECUTE_CHAR)
                                && i < buffer.len() - 1
                            {
                                let potential_snippet: String =
                                    buffer[i..].iter().map(|(ci, _)| *ci).collect();

                                if potential_snippet.contains('(')
                                    && !potential_snippet.contains(')')
                                {
                                    continue;
                                }

                                if let Ok(Some(expansion)) =
                                    process_expansion(&potential_snippet, &snippets_guard)
                                {
                                    let _ = handle_expansion(potential_snippet.len(), expansion);
                                    *just_expanded_val = true;
                                    buffer.drain(i..);
                                    return None;
                                }
                            }
                        }
                        let now = Instant::now();
                        buffer.retain(|(_, timestamp)| {
                            now.duration_since(*timestamp) < Duration::from_secs(10)
                        });
                        while buffer.len() > 100 {
                            buffer.remove(0);
                        }
                        Some(event)
                    } else {
                        Some(event)
                    }
                }
            }
        };

        // Register the callback
        if let Err(error) = rdev::grab(callback) {
            eprintln!("Error: Unable to grab keyboard events: {:?}. Ensure you have necessary permissions (e.g., member of 'input' group on Linux for Wayland/evdev).", error);
        }
    })
}
