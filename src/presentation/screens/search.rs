use ratatui::Frame;

use crate::application::form::SearchForm;

pub struct SearchScreen {
    pub form: SearchForm,
}

impl Default for SearchScreen {
    fn default() -> Self {
        let mut form = SearchForm::default();
        form.system = "Sol".to_owned();
        Self { form }
    }
}

pub fn render(_frame: &mut Frame<'_>, _screen: &SearchScreen) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_screen_shows_profiles_fields_summary_and_keyboard_hints() {
        let screen = SearchScreen::default();
        let backend = ratatui::backend::TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &screen)).unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Сценарий"));
        assert!(rendered.contains("Параметры"));
        assert!(rendered.contains("Ищем"));
        assert!(rendered.contains("Sol"));
        assert!(rendered.contains("F2"));
    }
}
