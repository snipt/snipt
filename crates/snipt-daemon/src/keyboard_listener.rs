use rdev::{self, EventType, Key as RdevKey};
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

    // Clone for the thread
    let snippets_clone = Arc::clone(&snippets);
    let running_clone = Arc::clone(&running);

    thread::spawn(move || {
        // Create a callback function closure
        let callback = move |event: rdev::Event| {
            if !*running_clone.lock().unwrap() {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => {
                    let mut buffer = buffer_clone.lock().unwrap();
                    let mut just_expanded = expanded_flag_clone.lock().unwrap();

                    // Handle special keys
                    match key {
                        RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                            // Check if we should expand the current buffer
                            if !buffer.is_empty() {
                                // Get the characters from the buffer, ignore timestamps
                                let buffer_text: String = buffer.iter().map(|(c, _)| *c).collect();

                                let snippets_guard = snippets_clone.lock().unwrap();
                                if let Ok(Some(expansion)) =
                                    process_expansion(&buffer_text, &snippets_guard)
                                {
                                    // Delete the special character and shortcut, then type the expanded text
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

                            let c = match key {
                                RdevKey::Space => ' ',
                                RdevKey::Return => '\n',
                                RdevKey::Tab => '\t',
                                _ => unreachable!(),
                            };
                            buffer.push((c, Instant::now()));
                        }
                        RdevKey::Backspace => {
                            if !buffer.is_empty() {
                                buffer.pop();
                            }
                            // Reset expansion flag if user is editing
                            *just_expanded = false;
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

                                // Check for snippet patterns in the buffer
                                let snippets_guard = snippets_clone.lock().unwrap();

                                // Look for ":snippet_name" patterns in the current buffer
                                for i in 0..buffer.len() {
                                    if buffer[i].0 == ':' && i < buffer.len() - 1 {
                                        // Extract potential snippet from this position onward
                                        let potential_snippet: String =
                                            buffer[i..].iter().map(|(c, _)| *c).collect();

                                        if let Ok(Some(expansion)) =
                                            process_expansion(&potential_snippet, &snippets_guard)
                                        {
                                            // Found a matching snippet to expand!

                                            // Calculate how many characters to delete (the snippet shortcut)
                                            let chars_to_delete = potential_snippet.len();

                                            // Delete the shortcut, then type the expanded text
                                            let _ =
                                                handle_expansion(chars_to_delete - 1, expansion);

                                            // Set flag that we just expanded
                                            *just_expanded = true;

                                            // Remove the expanded snippet from buffer
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

                                // Limit buffer size to prevent memory issues
                                if buffer.len() > 100 {
                                    buffer.remove(0);
                                }
                            }
                        }
                    }
                }
                EventType::KeyRelease(_) => {
                    // We need this to better handle key combinations
                    // but don't need to do anything here
                }
                _ => {}
            }
        };

        // Start a retry loop for the keyboard listener
        let mut retry_count = 0;
        let max_retries = 5;

        while *running.lock().unwrap() && retry_count < max_retries {
            match rdev::listen(callback.clone()) {
                Ok(_) => {
                    // Normally this shouldn't happen since listen() blocks
                    break;
                }
                Err(e) => {
                    eprintln!("Error in keyboard listener: {:?}", e);
                    retry_count += 1;
                    eprintln!(
                        "Retrying keyboard listener ({}/{})...",
                        retry_count, max_retries
                    );
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }

        if retry_count >= max_retries {
            eprintln!(
                "Failed to start keyboard listener after {} attempts",
                max_retries
            );
        }
    })
}
