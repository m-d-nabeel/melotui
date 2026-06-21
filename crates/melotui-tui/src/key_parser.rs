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

        let key_part;
        let mods_part;

        // Correctly isolate the keycode from the modifiers right-to-left
        if inner.ends_with('-') && inner.len() > 1 {
            // E.g. "C--" -> key is "-", modifiers are "C"
            key_part = "-";
            mods_part = &inner[..inner.len() - 2];
        } else if let Some(last_hyphen) = inner.rfind('-') {
            key_part = &inner[last_hyphen + 1..];
            mods_part = &inner[..last_hyphen];
        } else {
            key_part = inner;
            mods_part = "";
        }

        if !mods_part.is_empty() {
            for modifier in mods_part.split('-') {
                match modifier.to_lowercase().as_str() {
                    "c" | "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
                    "s" | "shift" => modifiers.insert(KeyModifiers::SHIFT),
                    "a" | "alt" | "m" | "meta" => modifiers.insert(KeyModifiers::ALT),
                    _ => return Err(format!("Unknown modifier: {}", modifier)),
                }
            }
        }

        let code = parse_keycode(key_part)?;

        // Auto apply shift conversions strictly for ASCII letters
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key() {
        // Basic letters
        assert_eq!(
            parse_key("j"),
            Ok((KeyCode::Char('j'), KeyModifiers::empty()))
        );
        assert_eq!(
            parse_key("J"),
            Ok((KeyCode::Char('j'), KeyModifiers::SHIFT))
        );

        // Modifiers with letters
        assert_eq!(
            parse_key("<C-c>"),
            Ok((KeyCode::Char('c'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("<A-Enter>"),
            Ok((KeyCode::Enter, KeyModifiers::ALT))
        );
        assert_eq!(parse_key("<S-Up>"), Ok((KeyCode::Up, KeyModifiers::SHIFT)));
        assert_eq!(
            parse_key("<C-Left>"),
            Ok((KeyCode::Left, KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("<C-S-A-Up>"),
            Ok((
                KeyCode::Up,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT
            ))
        );

        // Symbols (Layout agnostic - parses raw values without attempting translation)
        assert_eq!(
            parse_key("<S-.>"),
            Ok((KeyCode::Char('.'), KeyModifiers::SHIFT))
        );
        assert_eq!(
            parse_key("<S-,>"),
            Ok((KeyCode::Char(','), KeyModifiers::SHIFT))
        );

        // The minus sign edge case
        assert_eq!(
            parse_key("<S-->"),
            Ok((KeyCode::Char('-'), KeyModifiers::SHIFT))
        );
        assert_eq!(
            parse_key("<C-->"),
            Ok((KeyCode::Char('-'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("<C-S-->"),
            Ok((
                KeyCode::Char('-'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            ))
        );
    }
}
