use rdev::{self, EventType, Key as RdevKey};
use scribe_core::expansion::{process_expansion, replace_text};
use scribe_core::keyboard::rdev_key_to_char;
use std::sync::{Arc, Mutex};
use std::thread;

/// Starts listening for keyboard events and handles text expansion
pub fn start_keyboard_listener(
    snippets: Arc<Mutex<Vec<scribe_core::models::SnippetEntry>>>,
    running: Arc<Mutex<bool>>,
) -> thread::JoinHandle<()> {
    // Buffer for text accumulation
    let text_buffer = Arc::new(Mutex::new(String::new()));
    let buffer_clone = Arc::clone(&text_buffer);

    // Clone for the thread
    let snippets_clone = Arc::clone(&snippets);
    let running_clone = Arc::clone(&running);

    thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event| {
            if !*running_clone.lock().unwrap() {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => {
                    let mut buffer = buffer_clone.lock().unwrap();

                    match key {
                        RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                            // Check if we should expand the current buffer
                            if !buffer.is_empty() {
                                let snippets_guard = snippets_clone.lock().unwrap();
                                if let Ok(Some(expansion)) =
                                    process_expansion(&buffer, &snippets_guard)
                                {
                                    // Delete the special character and shortcut, then type the expanded text
                                    let _ = replace_text(buffer.len() + 1, &expansion);
                                }
                            }

                            // Clear buffer regardless of expansion
                            buffer.clear();
                        }
                        RdevKey::Backspace => {
                            if !buffer.is_empty() {
                                buffer.pop();
                            }
                        }
                        _ => {
                            // Add the character to our buffer
                            if let Some(c) = rdev_key_to_char(&key, &event) {
                                buffer.push(c);

                                // Limit buffer size to prevent memory issues
                                if buffer.len() > 100 {
                                    buffer.remove(0);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }) {
            eprintln!("Error setting up keyboard listener: {:?}", e);
        }
    })
}
