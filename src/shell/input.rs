use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Converts a crossterm KeyEvent to bytes that can be written to a PTY
pub fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Control characters (Ctrl-A = 0x01, Ctrl-B = 0x02, etc.)
                // Special case for Ctrl-@ (null) and some other control chars
                if c == ' ' || c == '@' {
                    vec![0x00]
                } else if c == '?' {
                    vec![0x7f]
                } else {
                    let byte = (c.to_ascii_uppercase() as u8) & 0x1f;
                    vec![byte]
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                // Alt modifier sends ESC followed by the character
                let mut bytes = vec![0x1b];
                bytes.extend(c.to_string().as_bytes());
                bytes
            } else {
                // Regular character
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                // Shift-Tab (reverse tab)
                b"\x1b[Z".to_vec()
            } else {
                vec![b'\t']
            }
        }
        KeyCode::Esc => vec![0x1b],

        // Arrow keys
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),

        // Home/End
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),

        // Page Up/Down
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),

        // Delete/Insert
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),

        // Function keys
        KeyCode::F(1) => b"\x1bOP".to_vec(),
        KeyCode::F(2) => b"\x1bOQ".to_vec(),
        KeyCode::F(3) => b"\x1bOR".to_vec(),
        KeyCode::F(4) => b"\x1bOS".to_vec(),
        KeyCode::F(5) => b"\x1b[15~".to_vec(),
        KeyCode::F(6) => b"\x1b[17~".to_vec(),
        KeyCode::F(7) => b"\x1b[18~".to_vec(),
        KeyCode::F(8) => b"\x1b[19~".to_vec(),
        KeyCode::F(9) => b"\x1b[20~".to_vec(),
        KeyCode::F(10) => b"\x1b[21~".to_vec(),
        KeyCode::F(11) => b"\x1b[23~".to_vec(),
        KeyCode::F(12) => b"\x1b[24~".to_vec(),

        // Unsupported keys return empty vec
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_char() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(event), b"a");
    }

    #[test]
    fn test_control_char() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(event), vec![0x03]); // Ctrl-C
    }

    #[test]
    fn test_enter() {
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(event), b"\r");
    }

    #[test]
    fn test_arrow_keys() {
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(up), b"\x1b[A");

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(down), b"\x1b[B");
    }
}
