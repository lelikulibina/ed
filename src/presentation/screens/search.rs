use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::application::{
    config::SearchConfig,
    form::{FieldError, SearchForm},
    presets::PRESETS,
};

use super::super::theme;

#[derive(Debug, Clone, PartialEq)]
pub enum SearchScreenAction {
    Start(SearchConfig),
    Exit,
    None,
}

pub struct SearchScreen {
    pub form: SearchForm,
    selected_preset: usize,
    advanced: bool,
    errors: Vec<FieldError>,
}

impl Default for SearchScreen {
    fn default() -> Self {
        let mut form = SearchForm {
            system: "Sol".to_owned(),
            ..SearchForm::default()
        };
        form.apply_preset(&PRESETS[0]);
        Self {
            form,
            selected_preset: 0,
            advanced: false,
            errors: Vec::new(),
        }
    }
}

impl SearchScreen {
    pub fn new(form: SearchForm) -> Self {
        Self {
            form,
            selected_preset: PRESETS.len(),
            advanced: false,
            errors: Vec::new(),
        }
    }

    pub fn errors(&self) -> &[FieldError] {
        &self.errors
    }

    pub fn handle_key(&mut self, _key: KeyEvent) -> SearchScreenAction {
        SearchScreenAction::None
    }

    pub fn handle_paste(&mut self, _text: &str) -> bool {
        false
    }

    pub fn from_config(config: &SearchConfig) -> Self {
        Self {
            form: SearchForm::from_config(config),
            selected_preset: PRESETS.len(),
            advanced: true,
            errors: Vec::new(),
        }
    }

    pub fn selected_preset(&self) -> usize {
        self.selected_preset
    }

    pub fn advanced(&self) -> bool {
        self.advanced
    }

    pub fn select_preset(&mut self, index: usize) {
        if index < PRESETS.len() {
            self.form.apply_preset(&PRESETS[index]);
            self.selected_preset = index;
        } else if index == PRESETS.len() {
            self.selected_preset = PRESETS.len();
        }
    }

    pub fn form_mut(&mut self) -> &mut SearchForm {
        self.selected_preset = PRESETS.len();
        &mut self.form
    }

    pub fn set_advanced(&mut self, expanded: bool) {
        self.advanced = expanded;
    }
}

pub fn render(frame: &mut Frame<'_>, screen: &SearchScreen) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("ED Planet Search")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL)),
        root[0],
    );

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(root[1]);

    let profiles = PRESETS
        .iter()
        .enumerate()
        .map(|(index, preset)| profile_item(preset.name, index == screen.selected_preset))
        .chain(std::iter::once(profile_item(
            "Свой поиск",
            screen.selected_preset == PRESETS.len(),
        )))
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(profiles).block(
            Block::default()
                .title(" Сценарий ")
                .borders(Borders::ALL)
                .border_style(theme::Theme::default().muted),
        ),
        columns[0],
    );

    let mut fields = vec![
        format!("Система             {}", screen.form.system),
        format!("Сколько             {}", screen.form.count),
        format!(
            "Без биосигналов     {}",
            if screen.form.exclude_with_bio_signals {
                "[x]"
            } else {
                "[ ]"
            }
        ),
    ];
    if screen.advanced {
        fields.extend([
            format!(
                "Поверхность         {}",
                screen
                    .form
                    .surface_types
                    .iter()
                    .map(|value| value.label_ru())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            format!(
                "Атмосфера           {}",
                screen
                    .form
                    .atmosphere_types
                    .iter()
                    .map(|value| value.label_ru())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            format!("Температура         {} K", screen.form.temperature),
            format!("Гравитация          {} g", screen.form.gravity),
            format!("Давление            {} atm", screen.form.pressure),
        ]);
    } else {
        fields.push("▸ Дополнительно (F2)".to_owned());
    }
    fields.push(String::new());
    fields.push(format!(
        "Потенциал профиля   {} млн CR",
        PRESETS
            .get(screen.selected_preset)
            .map(|preset| preset.potential_value_cr / 1_000_000)
            .unwrap_or_default()
    ));

    frame.render_widget(
        Paragraph::new(fields.join("\n"))
            .block(
                Block::default()
                    .title(" Параметры ")
                    .borders(Borders::ALL)
                    .border_style(theme::Theme::default().muted),
            )
            .wrap(Wrap { trim: false }),
        columns[1],
    );

    let summary = screen.form.summary().replace('\n', " · ");
    frame.render_widget(
        Paragraph::new(format!("Ищем: {summary} · радиус 1000 св. лет"))
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Поиск по критериям ")
                    .borders(Borders::ALL),
            ),
        root[2],
    );

    frame.render_widget(
        Paragraph::new(
            "Enter — искать · Tab — поле · F2 — дополнительно · ? — помощь · Esc — выход",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray)),
        root[3],
    );
}

fn profile_item(name: &'static str, selected: bool) -> ListItem<'static> {
    let prefix = if selected { "▸ " } else { "  " };
    ListItem::new(format!("{prefix}{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn rendered(screen: &SearchScreen, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, screen)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn selecting_a_profile_applies_it_immediately() {
        let mut screen = SearchScreen::default();
        let action = screen.handle_key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ));

        assert_eq!(action, SearchScreenAction::None);
        assert_eq!(screen.selected_preset(), 1);
        assert_eq!(screen.form.temperature, "180-190");
    }

    #[test]
    fn f2_expands_advanced_fields() {
        let mut screen = SearchScreen::default();
        screen.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));

        assert!(screen.advanced());
    }

    #[test]
    fn invalid_submission_keeps_input_and_reports_the_field_error() {
        let mut form = SearchForm::default();
        form.system = String::new();
        let mut screen = SearchScreen::new(form);
        let action = screen.handle_key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ));

        assert_eq!(action, SearchScreenAction::None);
        assert_eq!(screen.form.system, "");
        assert!(screen.errors().iter().any(|error| error.field == crate::application::form::FormField::System));
    }

    #[test]
    fn typing_and_pasting_into_the_system_field_updates_the_form() {
        let mut screen = SearchScreen::default();
        screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(screen.form.system, "SolX");

        assert!(screen.handle_paste(" Prime"));
        assert_eq!(screen.form.system, "SolX Prime");
        assert_eq!(screen.selected_preset(), PRESETS.len());
    }

    #[test]
    fn search_screen_shows_profiles_fields_summary_and_keyboard_hints() {
        let screen = SearchScreen::default();
        let rendered = rendered(&screen, 120, 40);

        assert!(rendered.contains("Сценарий"));
        assert!(rendered.contains("Параметры"));
        assert!(rendered.contains("Ищем"));
        assert!(rendered.contains("Sol"));
        assert!(rendered.contains("F2"));
        assert!(rendered.contains("Свой поиск"));
    }

    #[test]
    fn search_screen_remains_readable_at_minimum_supported_size() {
        let screen = SearchScreen::default();
        let rendered = rendered(&screen, 80, 24);

        assert!(rendered.contains("Сценарий"));
        assert!(rendered.contains("Ищем"));
        assert!(rendered.contains("Sol"));
    }

    #[test]
    fn choosing_a_preset_updates_the_form_and_custom_selection_keeps_values() {
        let mut screen = SearchScreen::default();
        screen.form.system = "Colonia".to_owned();
        screen.select_preset(2);

        assert_eq!(screen.form.system, "Colonia");
        assert_eq!(screen.form.temperature, "395-450");
        assert_eq!(screen.selected_preset(), 2);

        screen.select_preset(PRESETS.len());
        assert_eq!(screen.form.temperature, "395-450");
        assert_eq!(screen.selected_preset(), PRESETS.len());
    }
}
