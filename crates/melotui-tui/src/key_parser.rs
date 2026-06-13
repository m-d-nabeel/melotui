use ratatui::crossterm::event::{KeyCode, KeyModifiers};

/// Parses a Vim-style key string into a Crossterm KeyCode and KeyModifiers.
/// Examples:
/// "j" -> (KeyCode::Char('j'), KeyModifiers::empty())
/// "<C-c>" -> (KeyCode::Char('c'), KeyModifiers::CONTROL)
/// "<S-Up>" -> (KeyCode::Up, KeyModifiers::SHIFT)
/// "<A-Enter>" -> (KeyCode::Enter, KeyModifiers::ALT)
/// "<Space>" or "Space" -> (KeyCode::Char(' '), KeyModifiers::empty())
pub fn parse_key(s: &str) -> Result<(KeyCode, KeyModifiers), String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty key string".to_string());
    }

    // Handle generic single character
    if s.len() == 1 {
        let ch = s.chars().next().unwrap();
        // If it's a capital letter and we didn't specify a modifier, crossterm still considers it Char(c) + SHIFT
        if ch.is_ascii_uppercase() {
            return Ok((KeyCode::Char(ch.to_ascii_lowercase()), KeyModifiers::SHIFT));
        }
        return Ok((KeyCode::Char(ch), KeyModifiers::empty()));
    }

    // Handle full <M-c> or <C-S-Up> style
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        let mut modifiers = KeyModifiers::empty();
        let mut parts: Vec<&str> = inner.split('-').collect();

        let key_part = parts.pop().ok_or("Empty key brackets")?;

        for modifier in parts {
            match modifier.to_lowercase().as_str() {
                "c" | "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
                "s" | "shift" => modifiers.insert(KeyModifiers::SHIFT),
                "a" | "alt" | "m" | "meta" => modifiers.insert(KeyModifiers::ALT),
                _ => return Err(format!("Unknown modifier: {}", modifier)),
            }
        }

        let code = parse_keycode(key_part)?;
        // Auto apply shift for capital letters
        if let KeyCode::Char(c) = code
            && c.is_ascii_uppercase()
        {
            modifiers.insert(KeyModifiers::SHIFT);
            return Ok((KeyCode::Char(c.to_ascii_lowercase()), modifiers));
        }

        return Ok((code, modifiers));
    }

    // Fallback for names without brackets like "Enter", "Space"
    let code = parse_keycode(s)?;
    Ok((code, KeyModifiers::empty()))
}

fn parse_keycode(s: &str) -> Result<KeyCode, String> {
    match s.to_lowercase().as_str() {
        "enter" | "cr" | "return" => Ok(KeyCode::Enter),
        "esc" | "escape" => Ok(KeyCode::Esc),
        "space" | "spc" => Ok(KeyCode::Char(' ')),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "backspace" | "bs" => Ok(KeyCode::Backspace),
        "tab" => Ok(KeyCode::Tab),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" | "pgup" => Ok(KeyCode::PageUp),
        "pagedown" | "pgdn" => Ok(KeyCode::PageDown),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        _ => {
            if s.len() == 1 {
                Ok(KeyCode::Char(s.chars().next().unwrap()))
            } else if s.to_lowercase().starts_with('f') {
                if let Ok(num) = s[1..].parse::<u8>() {
                    Ok(KeyCode::F(num))
                } else {
                    Err(format!("Invalid function key: {}", s))
                }
            } else {
                Err(format!("Unknown key code: {}", s))
            }
        }
    }
}
