use crate::error::{Result, SniptError};
use enigo::Keyboard;
use enigo::{Direction, Enigo, Key, Settings};
use rdev::{self, Key as RdevKey};
use std::thread;
use std::time::Duration;

/// Convert rdev::Key to char
pub fn rdev_key_to_char(key: &RdevKey, event: &rdev::Event) -> Option<char> {
    // Handle special keys with custom transformations
    let special_char = match key {
        RdevKey::Kp0 if event.name == Some("!".to_string()) => Some('!'),
        RdevKey::Kp1 if event.name == Some("@".to_string()) => Some('@'),
        RdevKey::Kp2 if event.name == Some("#".to_string()) => Some('#'),
        RdevKey::Kp3 if event.name == Some("$".to_string()) => Some('$'),
        RdevKey::Kp4 if event.name == Some("%".to_string()) => Some('%'),
        RdevKey::Kp5 if event.name == Some("^".to_string()) => Some('^'),
        RdevKey::Kp6 if event.name == Some("&".to_string()) => Some('&'),
        RdevKey::Kp7 if event.name == Some("*".to_string()) => Some('*'),
        RdevKey::Kp8 if event.name == Some("(".to_string()) => Some('('),
        RdevKey::Kp9 if event.name == Some(")".to_string()) => Some(')'),
        RdevKey::KpMinus if event.name == Some("_".to_string()) => Some('_'),
        RdevKey::Equal if event.name == Some("+".to_string()) => Some('+'),
        RdevKey::SemiColon if event.name == Some(":".to_string()) => Some(':'),
        RdevKey::SemiColon if event.name == Some(";".to_string()) => Some(';'),
        RdevKey::Quote if event.name == Some("\"".to_string()) => Some('"'),
        RdevKey::Quote if event.name == Some("'".to_string()) => Some('\''),
        RdevKey::Comma if event.name == Some("<".to_string()) => Some('<'),
        RdevKey::Comma if event.name == Some(",".to_string()) => Some(','),
        RdevKey::Dot if event.name == Some(">".to_string()) => Some('>'),
        RdevKey::Dot if event.name == Some(".".to_string()) => Some('.'),
        RdevKey::Slash if event.name == Some("?".to_string()) => Some('?'),
        RdevKey::Slash if event.name == Some("/".to_string()) => Some('/'),
        RdevKey::BackSlash if event.name == Some("|".to_string()) => Some('|'),
        RdevKey::BackSlash if event.name == Some("\\".to_string()) => Some('\\'),
        _ => None,
    };

    if special_char.is_some() {
        return special_char;
    }

    // Regular single character keys
    if let Some(name) = &event.name {
        if name.len() == 1 {
            return name.chars().next();
        }
    }

    None
}

/// Create a keyboard controller
pub fn create_keyboard_controller() -> Result<Enigo> {
    // For Enigo 0.3.0 which requires Settings
    let settings = Settings::default();
    match Enigo::new(&settings) {
        Ok(enigo) => Ok(enigo),
        Err(err) => Err(SniptError::Enigo(format!(
            "Failed to create keyboard controller: {}",
            err
        ))),
    }
}

/// Type text using the keyboard controller
pub fn type_text(keyboard: &mut Enigo, text: &str) -> Result<()> {
    // For Enigo 0.3.0 which has a text method
    match keyboard.text(text) {
        Ok(_) => Ok(()),
        Err(err) => Err(SniptError::Enigo(format!("Failed to type text: {}", err))),
    }
}

/// Send backspace key presses
pub fn send_backspace(keyboard: &mut Enigo, count: usize) -> Result<()> {
    for _ in 0..count {
        // Reduced delay to speed up deletion
        thread::sleep(Duration::from_millis(2));

        // Use the key method with Direction::Click for Enigo 0.3.0
        match keyboard.key(Key::Backspace, Direction::Click) {
            Ok(_) => {}
            Err(err) => {
                return Err(SniptError::Enigo(format!(
                    "Failed to send backspace: {}",
                    err
                )))
            }
        }
    }
    Ok(())
}
