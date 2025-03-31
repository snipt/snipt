use rdev::{self, EventType, Key as RdevKey};
use snipt_core::expansion::{process_expansion, replace_text};
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
                                    let _ = replace_text(buffer_text.len() + 1, &expansion);
                                }
                            }

                            // Clear buffer regardless of expansion
                            buffer.clear();

                            // Add the space/newline/tab character if not expanded
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
                        }
                        _ => {
                            // Add the character to our buffer
                            if let Some(c) = rdev_key_to_char(&key, &event) {
                                buffer.push((c, Instant::now()));

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
