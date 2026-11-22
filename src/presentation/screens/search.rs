use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{
    application::{
        config::SearchConfig,
        form::{FieldError, FormField, SearchForm},
        presets::PRESETS,
    },
    domain::catalog::{AtmosphereType, SurfaceType},
};

use super::super::{text::ui, theme, widgets::text_input::TextInput};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchScreenAction {
    Start(SearchConfig),
    Exit,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchField {
    Profile,
    System,
    Count,
    BioSignals,
    Surface,
    Atmosphere,
    TemperatureMin,
    TemperatureMax,
    GravityMin,
    GravityMax,
    PressureMin,
    PressureMax,
}

pub struct SearchScreen {
    pub form: SearchForm,
    selected_preset: usize,
    advanced: bool,
    errors: Vec<FieldError>,
    focus: SearchField,
    system_input: TextInput,
    count_input: TextInput,
    temperature_min: TextInput,
    temperature_max: TextInput,
    gravity_min: TextInput,
    gravity_max: TextInput,
    pressure_min: TextInput,
    pressure_max: TextInput,
    surface_cursor: usize,
    atmosphere_cursor: usize,
    show_help: bool,
}

impl Default for SearchScreen {
    fn default() -> Self {
        let mut form = SearchForm {
            system: "Sol".to_owned(),
            ..SearchForm::default()
        };
        form.apply_preset(&PRESETS[0]);
        let mut screen = Self::new(form);
        screen.selected_preset = 0;
        screen
    }
}

impl SearchScreen {
    pub fn new(form: SearchForm) -> Self {
        let (temperature_min, temperature_max) = split_range(&form.temperature, "0", "5000");
        let (gravity_min, gravity_max) = split_range(&form.gravity, "0.1", "1.0");
        let (pressure_min, pressure_max) = split_range(&form.pressure, "0", "10");
        let surface_cursor = [
            SurfaceType::Any,
            SurfaceType::RockyBody,
            SurfaceType::IcyBody,
            SurfaceType::RockyIceWorld,
            SurfaceType::HighMetalContentWorld,
        ]
        .iter()
        .position(|surface| form.surface_types.contains(surface))
        .unwrap_or_default();
        let atmosphere_cursor = [
            AtmosphereType::Any,
            AtmosphereType::Ammonia,
            AtmosphereType::Water,
            AtmosphereType::CarbonDioxide,
            AtmosphereType::SulphurDioxide,
            AtmosphereType::Nitrogen,
            AtmosphereType::WaterRich,
            AtmosphereType::MethaneRich,
            AtmosphereType::CarbonDioxideRich,
            AtmosphereType::Methane,
            AtmosphereType::Helium,
            AtmosphereType::SilicateVapour,
            AtmosphereType::NeonRich,
            AtmosphereType::ArgonRich,
            AtmosphereType::Neon,
            AtmosphereType::Argon,
            AtmosphereType::Oxygen,
        ]
        .iter()
        .position(|atmosphere| form.atmosphere_types.contains(atmosphere))
        .unwrap_or_default();

        Self {
            system_input: TextInput::new(form.system.clone()),
            count_input: TextInput::new(form.count.clone()),
            temperature_min: TextInput::new(temperature_min),
            temperature_max: TextInput::new(temperature_max),
            gravity_min: TextInput::new(gravity_min),
            gravity_max: TextInput::new(gravity_max),
            pressure_min: TextInput::new(pressure_min),
            pressure_max: TextInput::new(pressure_max),
            form,
            selected_preset: PRESETS.len(),
            advanced: false,
            errors: Vec::new(),
            focus: SearchField::Profile,
            surface_cursor,
            atmosphere_cursor,
            show_help: false,
        }
    }

    pub fn from_config(config: &SearchConfig) -> Self {
        let mut screen = Self::new(SearchForm::from_config(config));
        screen.selected_preset = PRESETS
            .iter()
            .position(|preset| {
                preset
                    .to_criteria(
                        config.criteria.count,
                        config.criteria.exclude_with_bio_signals,
                    )
                    .is_ok_and(|criteria| criteria == config.criteria)
            })
            .unwrap_or(PRESETS.len());
        screen.advanced = true;
        screen
    }

    pub fn selected_preset(&self) -> usize {
        self.selected_preset
    }

    pub fn advanced(&self) -> bool {
        self.advanced
    }

    pub fn errors(&self) -> &[FieldError] {
        &self.errors
    }

    pub fn select_preset(&mut self, index: usize) {
        if index < PRESETS.len() {
            self.form.apply_preset(&PRESETS[index]);
            self.selected_preset = index;
            self.load_range_inputs();
            self.surface_cursor = (0..5)
                .find(|cursor| self.form.surface_types.contains(&surface_at(*cursor)))
                .unwrap_or_default();
            self.atmosphere_cursor = (0..17)
                .find(|cursor| self.form.atmosphere_types.contains(&atmosphere_at(*cursor)))
                .unwrap_or_default();
        } else if index == PRESETS.len() {
            self.selected_preset = index;
        }
        self.errors.clear();
    }

    pub fn set_advanced(&mut self, expanded: bool) {
        self.advanced = expanded;
        if !expanded && self.is_advanced_field() {
            self.focus = SearchField::Profile;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SearchScreenAction {
        if key.code == KeyCode::Esc {
            return SearchScreenAction::Exit;
        }
        if key.kind != KeyEventKind::Press {
            return SearchScreenAction::None;
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return SearchScreenAction::None;
            }
            KeyCode::F(2) => {
                self.set_advanced(!self.advanced);
                return SearchScreenAction::None;
            }
            KeyCode::Tab => {
                self.move_focus(1);
                return SearchScreenAction::None;
            }
            KeyCode::BackTab => {
                self.move_focus(-1);
                return SearchScreenAction::None;
            }
            KeyCode::Enter => {
                return match self.form.build() {
                    Ok(config) => {
                        self.errors.clear();
                        SearchScreenAction::Start(config)
                    }
                    Err(errors) => {
                        if let Some(first) = errors.first() {
                            self.focus = field_for_error(first.field);
                            if self.is_advanced_field() {
                                self.advanced = true;
                            }
                        }
                        self.errors = errors;
                        SearchScreenAction::None
                    }
                };
            }
            KeyCode::Up if self.focus == SearchField::Profile => {
                let next = if self.selected_preset == 0 {
                    PRESETS.len()
                } else {
                    self.selected_preset - 1
                };
                self.select_preset(next);
                return SearchScreenAction::None;
            }
            KeyCode::Down if self.focus == SearchField::Profile => {
                self.select_preset((self.selected_preset + 1) % (PRESETS.len() + 1));
                return SearchScreenAction::None;
            }
            KeyCode::Up | KeyCode::Down if self.focus == SearchField::Atmosphere => {
                self.move_atmosphere(if key.code == KeyCode::Up { -1 } else { 1 });
                return SearchScreenAction::None;
            }
            KeyCode::Up | KeyCode::Down if self.focus == SearchField::Surface => {
                self.move_surface(if key.code == KeyCode::Up { -1 } else { 1 });
                return SearchScreenAction::None;
            }
            KeyCode::Char(' ') if self.focus == SearchField::BioSignals => {
                self.form.exclude_with_bio_signals = !self.form.exclude_with_bio_signals;
                self.mark_custom();
                return SearchScreenAction::None;
            }
            KeyCode::Char(' ') if self.focus == SearchField::Surface => {
                self.toggle_surface();
                return SearchScreenAction::None;
            }
            _ => {}
        }

        let changed = match self.focus {
            SearchField::System => self.system_input.handle_key(key),
            SearchField::Count => self.count_input.handle_key(key),
            SearchField::TemperatureMin => self.temperature_min.handle_key(key),
            SearchField::TemperatureMax => self.temperature_max.handle_key(key),
            SearchField::GravityMin => self.gravity_min.handle_key(key),
            SearchField::GravityMax => self.gravity_max.handle_key(key),
            SearchField::PressureMin => self.pressure_min.handle_key(key),
            SearchField::PressureMax => self.pressure_max.handle_key(key),
            SearchField::Profile
            | SearchField::BioSignals
            | SearchField::Surface
            | SearchField::Atmosphere => false,
        };
        if changed {
            self.sync_form_from_inputs();
            self.mark_custom();
        }
        SearchScreenAction::None
    }

    pub fn handle_paste(&mut self, text: &str) -> bool {
        let changed = match self.focus {
            SearchField::System => self.system_input.paste(text),
            SearchField::Count => self.count_input.paste(text),
            SearchField::TemperatureMin => self.temperature_min.paste(text),
            SearchField::TemperatureMax => self.temperature_max.paste(text),
            SearchField::GravityMin => self.gravity_min.paste(text),
            SearchField::GravityMax => self.gravity_max.paste(text),
            SearchField::PressureMin => self.pressure_min.paste(text),
            SearchField::PressureMax => self.pressure_max.paste(text),
            SearchField::Profile
            | SearchField::BioSignals
            | SearchField::Surface
            | SearchField::Atmosphere => false,
        };
        if changed {
            self.sync_form_from_inputs();
            self.mark_custom();
        }
        changed
    }

    fn move_focus(&mut self, direction: isize) {
        let mut fields = vec![
            SearchField::Profile,
            SearchField::System,
            SearchField::Count,
            SearchField::BioSignals,
        ];
        if self.advanced {
            fields.extend([
                SearchField::Surface,
                SearchField::Atmosphere,
                SearchField::TemperatureMin,
                SearchField::TemperatureMax,
                SearchField::GravityMin,
                SearchField::GravityMax,
                SearchField::PressureMin,
                SearchField::PressureMax,
            ]);
        }
        let index = fields
            .iter()
            .position(|field| *field == self.focus)
            .unwrap_or_default();
        let next = (index as isize + direction).rem_euclid(fields.len() as isize) as usize;
        self.focus = fields[next];
    }

    fn move_surface(&mut self, direction: isize) {
        self.surface_cursor = (self.surface_cursor as isize + direction).rem_euclid(5) as usize;
    }

    fn move_atmosphere(&mut self, direction: isize) {
        self.atmosphere_cursor =
            (self.atmosphere_cursor as isize + direction).rem_euclid(17) as usize;
        self.form.atmosphere_types = vec![atmosphere_at(self.atmosphere_cursor)];
        self.mark_custom();
    }

    fn toggle_surface(&mut self) {
        let selected = surface_at(self.surface_cursor);
        if selected == SurfaceType::Any {
            self.form.surface_types = vec![SurfaceType::Any];
        } else {
            self.form
                .surface_types
                .retain(|surface| *surface != SurfaceType::Any);
            if let Some(index) = self
                .form
                .surface_types
                .iter()
                .position(|surface| *surface == selected)
            {
                self.form.surface_types.remove(index);
            } else {
                self.form.surface_types.push(selected);
            }
            if self.form.surface_types.is_empty() {
                self.form.surface_types = vec![SurfaceType::Any];
            }
        }
        self.mark_custom();
    }

    fn sync_form_from_inputs(&mut self) {
        self.form.system = self.system_input.value().to_owned();
        self.form.count = self.count_input.value().to_owned();
        self.form.temperature = format!(
            "{}-{}",
            self.temperature_min.value(),
            self.temperature_max.value()
        );
        self.form.gravity = format!("{}-{}", self.gravity_min.value(), self.gravity_max.value());
        self.form.pressure = format!(
            "{}-{}",
            self.pressure_min.value(),
            self.pressure_max.value()
        );
        self.errors.clear();
    }

    fn load_range_inputs(&mut self) {
        let (a, b) = split_range(&self.form.temperature, "0", "5000");
        self.temperature_min = TextInput::new(a);
        self.temperature_max = TextInput::new(b);
        let (a, b) = split_range(&self.form.gravity, "0.1", "1.0");
        self.gravity_min = TextInput::new(a);
        self.gravity_max = TextInput::new(b);
        let (a, b) = split_range(&self.form.pressure, "0", "10");
        self.pressure_min = TextInput::new(a);
        self.pressure_max = TextInput::new(b);
    }

    fn mark_custom(&mut self) {
        self.selected_preset = PRESETS.len();
        self.errors.clear();
    }

    fn is_advanced_field(&self) -> bool {
        matches!(
            self.focus,
            SearchField::Surface
                | SearchField::Atmosphere
                | SearchField::TemperatureMin
                | SearchField::TemperatureMax
                | SearchField::GravityMin
                | SearchField::GravityMax
                | SearchField::PressureMin
                | SearchField::PressureMax
        )
    }
}

fn split_range(value: &str, default_min: &str, default_max: &str) -> (String, String) {
    value
        .split_once('-')
        .map(|(min, max)| (min.to_owned(), max.to_owned()))
        .unwrap_or_else(|| (default_min.to_owned(), default_max.to_owned()))
}

fn surface_at(index: usize) -> SurfaceType {
    match index {
        0 => SurfaceType::Any,
        1 => SurfaceType::RockyBody,
        2 => SurfaceType::IcyBody,
        3 => SurfaceType::RockyIceWorld,
        _ => SurfaceType::HighMetalContentWorld,
    }
}

fn atmosphere_at(index: usize) -> AtmosphereType {
    match index {
        0 => AtmosphereType::Any,
        1 => AtmosphereType::Ammonia,
        2 => AtmosphereType::Water,
        3 => AtmosphereType::CarbonDioxide,
        4 => AtmosphereType::SulphurDioxide,
        5 => AtmosphereType::Nitrogen,
        6 => AtmosphereType::WaterRich,
        7 => AtmosphereType::MethaneRich,
        8 => AtmosphereType::CarbonDioxideRich,
        9 => AtmosphereType::Methane,
        10 => AtmosphereType::Helium,
        11 => AtmosphereType::SilicateVapour,
        12 => AtmosphereType::NeonRich,
        13 => AtmosphereType::ArgonRich,
        14 => AtmosphereType::Neon,
        15 => AtmosphereType::Argon,
        _ => AtmosphereType::Oxygen,
    }
}

fn field_for_error(field: FormField) -> SearchField {
    match field {
        FormField::System => SearchField::System,
        FormField::Count => SearchField::Count,
        FormField::Surface => SearchField::Surface,
        FormField::Atmosphere => SearchField::Atmosphere,
        FormField::Temperature => SearchField::TemperatureMin,
        FormField::Gravity => SearchField::GravityMin,
        FormField::Pressure => SearchField::PressureMin,
    }
}

pub fn render(frame: &mut Frame<'_>, screen: &SearchScreen) {
    let area = frame.area();
    if !theme::supports_minimum_size(area) {
        frame.render_widget(
            Paragraph::new(ui::MIN_TERMINAL_SIZE)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }

    if screen.show_help {
        frame.render_widget(
            Paragraph::new(ui::HELP_SEARCH)
                .wrap(Wrap { trim: true })
                .block(Block::default().title(ui::HELP_TITLE).borders(Borders::ALL)),
            theme::centered(
                area,
                area.width.saturating_sub(4),
                area.height.saturating_sub(4),
            ),
        );
        return;
    }
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(ui::APP_TITLE)
            .alignment(Alignment::Center)
            .style(theme::Theme::default().accent)
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
        .map(|(index, preset)| {
            profile_item(
                format!(
                    "{} · {} {} · {} {}",
                    preset.name,
                    preset.expensive_species,
                    ui::SEARCH_SPECIES_LABEL,
                    preset.potential_value_cr / 1_000_000,
                    ui::SEARCH_PROFILE_VALUE_UNIT
                ),
                index == screen.selected_preset,
            )
        })
        .chain(std::iter::once(profile_item(
            ui::SEARCH_CUSTOM_PROFILE.to_owned(),
            screen.selected_preset == PRESETS.len(),
        )))
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(profiles).block(
            Block::default()
                .title(ui::SEARCH_PROFILE_TITLE)
                .borders(Borders::ALL)
                .border_style(theme::Theme::default().muted),
        ),
        columns[0],
    );

    let mut fields = vec![
        field_line(
            format!(
                "{}: {}",
                ui::SEARCH_SYSTEM_LABEL,
                display_input(&screen.system_input, screen.focus == SearchField::System)
            ),
            screen.focus == SearchField::System,
        ),
        field_line(
            format!(
                "{}: {}",
                ui::SEARCH_COUNT_LABEL,
                display_input(&screen.count_input, screen.focus == SearchField::Count)
            ),
            screen.focus == SearchField::Count,
        ),
        field_line(
            format!(
                "{}: {}",
                ui::SEARCH_BIO_LABEL,
                if screen.form.exclude_with_bio_signals {
                    "[x]"
                } else {
                    "[ ]"
                }
            ),
            screen.focus == SearchField::BioSignals,
        ),
    ];
    push_error(&mut fields, screen, FormField::System);
    push_error(&mut fields, screen, FormField::Count);

    if screen.advanced {
        fields.extend([
            field_line(
                format!(
                    "{}         {} · {}: {}",
                    ui::SEARCH_SURFACE_LABEL,
                    screen
                        .form
                        .surface_types
                        .iter()
                        .map(|value| value.label_ru())
                        .collect::<Vec<_>>()
                        .join(", "),
                    ui::SEARCH_SURFACE_CHOICE_LABEL,
                    surface_at(screen.surface_cursor).label_ru()
                ),
                screen.focus == SearchField::Surface,
            ),
            field_line(
                format!(
                    "{}           {}",
                    ui::SEARCH_ATMOSPHERE_LABEL,
                    screen
                        .form
                        .atmosphere_types
                        .iter()
                        .map(|value| value.label_ru())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                screen.focus == SearchField::Atmosphere,
            ),
            field_line(
                format!(
                    "{}         {} — {} K",
                    ui::SEARCH_TEMPERATURE_LABEL,
                    display_input(
                        &screen.temperature_min,
                        screen.focus == SearchField::TemperatureMin
                    ),
                    display_input(
                        &screen.temperature_max,
                        screen.focus == SearchField::TemperatureMax
                    )
                ),
                matches!(
                    screen.focus,
                    SearchField::TemperatureMin | SearchField::TemperatureMax
                ),
            ),
            field_line(
                format!(
                    "{}          {} — {} g",
                    ui::SEARCH_GRAVITY_LABEL,
                    display_input(&screen.gravity_min, screen.focus == SearchField::GravityMin),
                    display_input(&screen.gravity_max, screen.focus == SearchField::GravityMax)
                ),
                matches!(
                    screen.focus,
                    SearchField::GravityMin | SearchField::GravityMax
                ),
            ),
            field_line(
                format!(
                    "{}            {} — {} atm",
                    ui::SEARCH_PRESSURE_LABEL,
                    display_input(
                        &screen.pressure_min,
                        screen.focus == SearchField::PressureMin
                    ),
                    display_input(
                        &screen.pressure_max,
                        screen.focus == SearchField::PressureMax
                    )
                ),
                matches!(
                    screen.focus,
                    SearchField::PressureMin | SearchField::PressureMax
                ),
            ),
        ]);
        push_error(&mut fields, screen, FormField::Surface);
        push_error(&mut fields, screen, FormField::Atmosphere);
        push_error(&mut fields, screen, FormField::Temperature);
        push_error(&mut fields, screen, FormField::Gravity);
        push_error(&mut fields, screen, FormField::Pressure);
    } else {
        fields.push(Line::from(format!("▸ {}", ui::SEARCH_ADVANCED_LABEL)));
    }
    fields.push(Line::from(""));
    fields.push(Line::from(format!(
        "{}   {} {}",
        ui::SEARCH_PROFILE_VALUE_LABEL,
        PRESETS
            .get(screen.selected_preset)
            .map(|preset| preset.potential_value_cr / 1_000_000)
            .unwrap_or_default(),
        ui::SEARCH_PROFILE_VALUE_UNIT
    )));

    frame.render_widget(
        Paragraph::new(Text::from(fields))
            .block(
                Block::default()
                    .title(ui::SEARCH_PARAMETERS_TITLE)
                    .borders(Borders::ALL)
                    .border_style(theme::Theme::default().muted),
            )
            .wrap(Wrap { trim: false }),
        columns[1],
    );

    let summary = format!(
        "{}: {} · {}: {}",
        ui::SEARCH_SUMMARY_PREFIX,
        screen.form.summary().replace('\n', " · "),
        ui::SEARCH_RADIUS_LABEL,
        ui::SEARCH_RADIUS_VALUE
    );
    frame.render_widget(
        Paragraph::new(summary).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(ui::SEARCH_SUMMARY_TITLE)
                .borders(Borders::ALL),
        ),
        root[2],
    );

    frame.render_widget(
        Paragraph::new(ui::SEARCH_HINTS)
            .alignment(Alignment::Center)
            .style(theme::Theme::default().muted),
        root[3],
    );
}

fn field_line(content: String, focused: bool) -> Line<'static> {
    let style = if focused {
        theme::Theme::default().accent.add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(Span::styled(content, style))
}

fn display_input(input: &TextInput, focused: bool) -> String {
    if !focused {
        return input.value().to_owned();
    }
    let mut characters = input.value().chars().collect::<Vec<_>>();
    characters.insert(input.cursor(), '▏');
    characters.into_iter().collect()
}

fn push_error(fields: &mut Vec<Line<'static>>, screen: &SearchScreen, field: FormField) {
    if let Some(error) = screen.errors.iter().find(|error| error.field == field) {
        fields.push(Line::from(Span::styled(
            format!("{}{}", ui::FORM_ERROR_MARKER, error.message),
            theme::Theme::default().error,
        )));
    }
}

fn profile_item(name: String, selected: bool) -> ListItem<'static> {
    let prefix = if selected { "▸ " } else { "  " };
    let style = if selected {
        theme::Theme::default().accent
    } else {
        Style::default()
    };
    ListItem::new(format!("{prefix}{name}")).style(style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        let action = screen.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

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
        let form = SearchForm {
            system: String::new(),
            ..SearchForm::default()
        };
        let mut screen = SearchScreen::new(form);
        let action = screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(action, SearchScreenAction::None);
        assert_eq!(screen.form.system, "");
        assert!(
            screen
                .errors()
                .iter()
                .any(|error| error.field == crate::application::form::FormField::System)
        );
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
    fn cli_configuration_prefills_the_screen_without_changing_criteria() {
        let config = SearchForm {
            system: "Colonia".to_owned(),
            surface_types: vec![SurfaceType::RockyBody, SurfaceType::HighMetalContentWorld],
            atmosphere_types: vec![AtmosphereType::Water],
            temperature: "395-450".to_owned(),
            gravity: "0.1-1.0".to_owned(),
            pressure: "0.040-0.063".to_owned(),
            count: "6".to_owned(),
            exclude_with_bio_signals: true,
        }
        .build()
        .unwrap();

        let screen = SearchScreen::from_config(&config);

        assert_eq!(screen.form.build().unwrap(), config);
        assert_eq!(screen.selected_preset(), 2);
        assert!(screen.advanced());
    }

    #[test]
    fn selected_profile_preserves_or_surfaces_and_gravity_range() {
        let mut screen = SearchScreen::default();
        screen.select_preset(2);
        let config = screen.form.build().unwrap();

        assert_eq!(
            config.criteria.surface,
            crate::domain::criteria::Filter::one_of(vec![
                SurfaceType::RockyBody,
                SurfaceType::HighMetalContentWorld,
            ])
        );
        assert_eq!(
            config.criteria.gravity,
            crate::domain::criteria::NumericRange { min: 0.1, max: 1.0 }
        );
    }

    #[test]
    fn returning_from_search_restores_the_matching_profile() {
        let original = SearchScreen::default();
        let config = original.form.build().unwrap();

        let restored = SearchScreen::from_config(&config);

        assert_eq!(restored.selected_preset(), 0);
        assert_eq!(restored.form, original.form);
    }

    #[test]
    fn profile_one_preserves_temperature_pressure_and_bio_switch() {
        let mut screen = SearchScreen::default();
        screen.form.exclude_with_bio_signals = true;
        let config = screen.form.build().unwrap();

        assert_eq!(
            config.criteria.temperature,
            crate::domain::criteria::NumericRange {
                min: 190.0,
                max: 195.0
            }
        );
        assert_eq!(
            config.criteria.pressure,
            crate::domain::criteria::NumericRange {
                min: 0.042,
                max: 0.237
            }
        );
        assert!(config.criteria.exclude_with_bio_signals);
    }

    #[test]
    fn advanced_temperature_and_gravity_bounds_are_independent_inputs() {
        let mut screen = SearchScreen::default();
        screen.set_advanced(true);
        for _ in 0..6 {
            screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }
        screen.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE));
        assert_eq!(screen.form.temperature, "1900-195");

        screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
        assert_eq!(screen.form.temperature, "1900-1955");
    }

    #[test]
    fn atmosphere_selection_cycles_to_the_next_typed_value() {
        let mut screen = SearchScreen::default();
        screen.set_advanced(true);
        for _ in 0..5 {
            screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }
        screen.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert_eq!(
            screen.form.atmosphere_types,
            vec![AtmosphereType::SulphurDioxide]
        );
    }

    #[test]
    fn enter_returns_a_valid_search_configuration() {
        let mut screen = SearchScreen::default();

        let action = screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(action, SearchScreenAction::Start(config) if config.system == "Sol"));
        assert!(screen.errors().is_empty());
    }

    #[test]
    fn surface_selection_supports_or_and_any_is_exclusive() {
        let mut screen = SearchScreen::new(SearchForm::default());
        screen.set_advanced(true);
        for _ in 0..4 {
            screen.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }

        screen.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(
            screen.form.surface_types,
            vec![SurfaceType::RockyBody, SurfaceType::IcyBody]
        );

        screen.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(screen.form.surface_types, vec![SurfaceType::IcyBody]);

        screen.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        screen.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(screen.form.surface_types, vec![SurfaceType::Any]);
    }

    #[test]
    fn invalid_submission_renders_the_field_error_in_the_screen() {
        let form = SearchForm {
            system: String::new(),
            ..SearchForm::default()
        };
        let mut screen = SearchScreen::new(form);
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(rendered(&screen, 120, 40).contains("Укажите центральную систему"));
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
    fn undersized_terminal_shows_only_the_resize_message() {
        let screen = SearchScreen::default();
        let rendered = rendered(&screen, 60, 18);

        assert!(rendered.contains("Увеличьте окно терминала"));
        assert!(!rendered.contains("Сценарий"));
    }

    #[test]
    fn question_mark_opens_and_closes_the_keyboard_help() {
        let mut screen = SearchScreen::default();
        screen.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(rendered(&screen, 120, 40).contains("Shift+Tab"));
        let help = rendered(&screen, 120, 40);
        assert!(help.contains("Пробел"));

        screen.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(rendered(&screen, 120, 40).contains("Сценарий"));
    }

    #[test]
    fn search_screen_remains_readable_at_minimum_and_wide_sizes() {
        let screen = SearchScreen::default();

        for (width, height) in [(80, 24), (120, 40)] {
            let output = rendered(&screen, width, height);

            assert!(
                output.contains("Сценарий"),
                "missing profile list at {width}x{height}"
            );
            assert!(
                output.contains("Ищем"),
                "missing criteria summary at {width}x{height}"
            );
            assert!(
                output.contains("Sol"),
                "missing default system at {width}x{height}"
            );
            assert!(
                output.contains("Количество: 5"),
                "missing result count at {width}x{height}"
            );
            assert!(
                output.contains("Радиус: 1000 св. лет"),
                "missing radius at {width}x{height}"
            );
        }
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
