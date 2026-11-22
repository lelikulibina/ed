use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

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

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }

        let mut characters = self.value.chars().collect::<Vec<_>>();
        match key.code {
            KeyCode::Char(character) if !character.is_control() => {
                characters.insert(self.cursor, character);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                characters.remove(self.cursor - 1);
                self.cursor -= 1;
            }
            KeyCode::Delete if self.cursor < characters.len() => {
                characters.remove(self.cursor);
            }
            KeyCode::Left if self.cursor > 0 => self.cursor -= 1,
            KeyCode::Right if self.cursor < characters.len() => self.cursor += 1,
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = characters.len(),
            _ => return false,
        }

        self.value = characters.into_iter().collect();
        true
    }

    pub fn paste(&mut self, text: &str) -> bool {
        let pasted = text.chars().filter(|character| !character.is_control()).collect::<Vec<_>>();
        if pasted.is_empty() {
            return false;
        }

        let mut characters = self.value.chars().collect::<Vec<_>>();
        let pasted_len = pasted.len();
        characters.splice(self.cursor..self.cursor, pasted);
        self.cursor += pasted_len;
        self.value = characters.into_iter().collect();
        true
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
