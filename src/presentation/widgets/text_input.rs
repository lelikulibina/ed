use crossterm::event::KeyEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    pub fn paste(&mut self, _text: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn inserts_text_at_the_cursor_using_character_positions() {
        let mut input = TextInput::new("Сол");
        assert!(input.handle_key(key(KeyCode::Left)));
        assert!(input.handle_key(key(KeyCode::Char('X'))));

        assert_eq!(input.value(), "СоXл");
        assert_eq!(input.cursor(), 3);
    }

    #[test]
    fn home_end_backspace_and_delete_edit_the_expected_character() {
        let mut input = TextInput::new("abc");
        assert!(input.handle_key(key(KeyCode::Home)));
        assert!(input.handle_key(key(KeyCode::Delete)));
        assert_eq!(input.value(), "bc");

        assert!(input.handle_key(key(KeyCode::End)));
        assert!(input.handle_key(key(KeyCode::Backspace)));
        assert_eq!(input.value(), "b");
        assert_eq!(input.cursor(), 1);
    }

    #[test]
    fn paste_inserts_text_at_cursor_and_ignores_control_characters() {
        let mut input = TextInput::new("Sol");
        assert!(input.handle_key(key(KeyCode::Left)));
        assert!(input.paste("✨\nX"));

        assert_eq!(input.value(), "So✨Xl");
        assert_eq!(input.cursor(), 4);
    }
}
