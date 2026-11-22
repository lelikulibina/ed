use std::io::{self, Stdout};

use anyhow::{Context, Result, anyhow};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    application::{
        config::SearchConfig,
        presets::{PRESETS, SearchPreset},
    },
    cli::{parse_range_f64, parse_range_i32},
    domain::{
        catalog::{AtmosphereType, SurfaceType},
        criteria::{Filter, NumericRange, SearchCriteria},
    },
};

const SURFACES: [&str; 5] = [
    SurfaceType::Any.spansh_value(),
    SurfaceType::RockyBody.spansh_value(),
    SurfaceType::IcyBody.spansh_value(),
    SurfaceType::RockyIceWorld.spansh_value(),
    SurfaceType::HighMetalContentWorld.spansh_value(),
];
const ATMOSPHERES: [&str; 17] = [
    AtmosphereType::Any.spansh_value(),
    AtmosphereType::Ammonia.spansh_value(),
    AtmosphereType::Water.spansh_value(),
    AtmosphereType::CarbonDioxide.spansh_value(),
    AtmosphereType::SulphurDioxide.spansh_value(),
    AtmosphereType::Nitrogen.spansh_value(),
    AtmosphereType::WaterRich.spansh_value(),
    AtmosphereType::MethaneRich.spansh_value(),
    AtmosphereType::CarbonDioxideRich.spansh_value(),
    AtmosphereType::Methane.spansh_value(),
    AtmosphereType::Helium.spansh_value(),
    AtmosphereType::SilicateVapour.spansh_value(),
    AtmosphereType::NeonRich.spansh_value(),
    AtmosphereType::ArgonRich.spansh_value(),
    AtmosphereType::Neon.spansh_value(),
    AtmosphereType::Argon.spansh_value(),
    AtmosphereType::Oxygen.spansh_value(),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Profiles,
    General,
    Filters,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditField {
    System,
    Count,
    Temperature,
    Gravity,
    Pressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceField {
    Surface,
    Atmosphere,
}

#[derive(Debug, Clone)]
struct Editor {
    field: EditField,
    value: String,
    cursor: usize,
}

#[derive(Debug, Clone)]
struct Choice {
    field: ChoiceField,
    cursor: usize,
    selected: Vec<bool>,
}

#[derive(Debug, Clone)]
enum Modal {
    Edit(Editor),
    Choice(Choice),
}

struct App {
    step: Step,
    profile: usize,
    row: usize,
    system: String,
    count: String,
    bio: bool,
    surfaces: Vec<String>,
    atmosphere: String,
    temperature: String,
    gravity: String,
    pressure: String,
    modal: Option<Modal>,
    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            step: Step::Profiles,
            profile: 0,
            row: 0,
            system: "Sol".into(),
            count: "5".into(),
            bio: false,
            surfaces: vec!["any".into()],
            atmosphere: "any".into(),
            temperature: "0-5000".into(),
            gravity: "0.1-1.0".into(),
            pressure: "0-10".into(),
            modal: None,
            error: None,
        }
    }
}

impl App {
    fn from_config(config: SearchConfig) -> Self {
        let criteria = config.criteria;
        let surfaces = match criteria.surface {
            Filter::Any => vec![SurfaceType::Any.spansh_value().to_owned()],
            Filter::OneOf(types) => types
                .into_iter()
                .map(|surface| surface.spansh_value().to_owned())
                .collect(),
        };
        let atmosphere = match criteria.atmosphere {
            Filter::Any => AtmosphereType::Any.spansh_value().to_owned(),
            Filter::OneOf(types) => types
                .first()
                .copied()
                .unwrap_or(AtmosphereType::Any)
                .spansh_value()
                .to_owned(),
        };

        Self {
            step: Step::General,
            profile: PRESETS.len(),
            row: 0,
            system: config.system,
            count: criteria.count.to_string(),
            bio: criteria.exclude_with_bio_signals,
            surfaces,
            atmosphere,
            temperature: format!("{}-{}", criteria.temperature.min, criteria.temperature.max),
            gravity: format!("{}-{}", criteria.gravity.min, criteria.gravity.max),
            pressure: format!("{}-{}", criteria.pressure.min, criteria.pressure.max),
            modal: None,
            error: None,
        }
    }

    fn choose_profile(&mut self) {
        if self.profile < PRESETS.len() {
            let p = &PRESETS[self.profile];
            self.surfaces = p
                .surface_types
                .iter()
                .map(|surface| surface.spansh_value().to_owned())
                .collect();
            self.atmosphere = p.atmosphere.spansh_value().to_owned();
            self.temperature = format!("{:.0}-{:.0}", p.temperature.min, p.temperature.max);
            self.gravity = "0.1-1.0".into();
            self.pressure = format!("{:.3}-{:.3}", p.pressure.min, p.pressure.max);
        } else {
            self.surfaces = vec!["any".into()];
            self.atmosphere = "any".into();
            self.temperature = "0-5000".into();
            self.gravity = "0.1-1.0".into();
            self.pressure = "0-10".into();
        }
        self.step = Step::General;
        self.row = 0;
        self.error = None;
    }

    fn row_count(&self) -> usize {
        match self.step {
            Step::Profiles => PRESETS.len() + 1,
            Step::General => 4,
            Step::Filters => 7,
            Step::Review => 2,
        }
    }

    fn move_row(&mut self, delta: isize) {
        let count = self.row_count();
        if count > 0 {
            self.row = (self.row as isize + delta).rem_euclid(count as isize) as usize;
            if self.step == Step::Profiles {
                self.profile = self.row;
            }
        }
        self.error = None;
    }

    fn open_editor(&mut self, field: EditField) {
        let value = self.edit_value(field).to_owned();
        self.modal = Some(Modal::Edit(Editor {
            field,
            cursor: value.chars().count(),
            value,
        }));
    }

    fn edit_value(&self, field: EditField) -> &str {
        match field {
            EditField::System => &self.system,
            EditField::Count => &self.count,
            EditField::Temperature => &self.temperature,
            EditField::Gravity => &self.gravity,
            EditField::Pressure => &self.pressure,
        }
    }

    fn save_editor(&mut self, editor: Editor) {
        match editor.field {
            EditField::System => self.system = editor.value,
            EditField::Count => self.count = editor.value,
            EditField::Temperature => self.temperature = editor.value,
            EditField::Gravity => self.gravity = editor.value,
            EditField::Pressure => self.pressure = editor.value,
        }
        self.error = None;
    }

    fn open_choice(&mut self, field: ChoiceField) {
        let options: &[&str] = match field {
            ChoiceField::Surface => &SURFACES,
            ChoiceField::Atmosphere => &ATMOSPHERES,
        };
        let selected = options
            .iter()
            .map(|option| match field {
                ChoiceField::Surface => self.surfaces.iter().any(|v| v == option),
                ChoiceField::Atmosphere => self.atmosphere == *option,
            })
            .collect::<Vec<_>>();
        let cursor = selected.iter().position(|v| *v).unwrap_or(0);
        self.modal = Some(Modal::Choice(Choice {
            field,
            cursor,
            selected,
        }));
    }

    fn save_choice(&mut self, choice: Choice) {
        let options: &[&str] = match choice.field {
            ChoiceField::Surface => &SURFACES,
            ChoiceField::Atmosphere => &ATMOSPHERES,
        };
        let values = options
            .iter()
            .zip(choice.selected.iter())
            .filter_map(|(value, selected)| selected.then_some((*value).to_owned()))
            .collect::<Vec<_>>();
        match choice.field {
            ChoiceField::Surface => {
                self.surfaces = if values.is_empty() {
                    vec!["any".into()]
                } else {
                    values
                };
                if self.surfaces.len() > 1 {
                    self.surfaces.retain(|v| v != "any");
                }
            }
            ChoiceField::Atmosphere => {
                self.atmosphere = values.first().cloned().unwrap_or_else(|| "any".into());
            }
        }
        self.error = None;
    }

    fn toggle_choice(choice: &mut Choice) {
        let index = choice.cursor;
        if choice.field == ChoiceField::Atmosphere {
            choice.selected.fill(false);
            choice.selected[index] = true;
            return;
        }
        if index == 0 {
            choice.selected.fill(false);
            choice.selected[0] = true;
        } else {
            choice.selected[0] = false;
            choice.selected[index] = !choice.selected[index];
            if !choice.selected.iter().any(|v| *v) {
                choice.selected[0] = true;
            }
        }
    }

    fn selection(&self) -> Result<SearchConfig> {
        if self.system.trim().is_empty() {
            return Err(anyhow!("Укажите центральную систему"));
        }
        let count = self
            .count
            .trim()
            .parse::<usize>()
            .context("Количество результатов должно быть целым числом")?;
        if count == 0 {
            return Err(anyhow!("Количество результатов должно быть больше нуля"));
        }
        let (t_min, t_max) = parse_range_i32(&self.temperature)
            .context("Температура: ожидается диапазон min-max в K")?;
        let (g_min, g_max) =
            parse_range_f64(&self.gravity).context("Гравитация: ожидается диапазон min-max в g")?;
        let (p_min, p_max) = parse_range_f64(&self.pressure)
            .context("Давление: ожидается диапазон min-max в atm")?;

        let surface_types = self
            .surfaces
            .iter()
            .map(|value| value.parse::<SurfaceType>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let surface = if surface_types.is_empty() || surface_types.contains(&SurfaceType::Any) {
            Filter::Any
        } else {
            Filter::one_of(surface_types)
        };
        let atmosphere_type = self.atmosphere.parse::<AtmosphereType>()?;
        let atmosphere = if atmosphere_type == AtmosphereType::Any {
            Filter::Any
        } else {
            Filter::one_of(vec![atmosphere_type])
        };

        let criteria = SearchCriteria::new(
            surface,
            atmosphere,
            NumericRange::new(t_min.into(), t_max.into())
                .ok_or_else(|| anyhow!("Некорректная температура"))?,
            NumericRange::new(g_min, g_max).ok_or_else(|| anyhow!("Некорректная гравитация"))?,
            NumericRange::new(p_min, p_max).ok_or_else(|| anyhow!("Некорректное давление"))?,
            count,
        )?
        .with_bio_signal_exclusion(self.bio);
        SearchConfig::new(self.system.clone(), criteria).map_err(Into::into)
    }

    fn advance(&mut self) {
        match self.step {
            Step::Profiles => self.choose_profile(),
            Step::General => {
                self.step = Step::Filters;
                self.row = 0;
            }
            Step::Filters => match self.selection() {
                Ok(_) => {
                    self.step = Step::Review;
                    self.row = 0;
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            Step::Review => {}
        }
    }

    fn back(&mut self) -> bool {
        match self.step {
            Step::Profiles => return false,
            Step::General => self.step = Step::Profiles,
            Step::Filters => self.step = Step::General,
            Step::Review => self.step = Step::Filters,
        }
        self.row = if self.step == Step::Profiles {
            self.profile
        } else {
            0
        };
        self.error = None;
        true
    }

    fn activate(&mut self) {
        match self.step {
            Step::Profiles => self.choose_profile(),
            Step::General => match self.row {
                0 => self.open_editor(EditField::System),
                1 => self.open_editor(EditField::Count),
                2 => self.bio = !self.bio,
                _ => self.advance(),
            },
            Step::Filters => match self.row {
                0 => self.open_choice(ChoiceField::Surface),
                1 => self.open_choice(ChoiceField::Atmosphere),
                2 => self.open_editor(EditField::Temperature),
                3 => self.open_editor(EditField::Gravity),
                4 => self.open_editor(EditField::Pressure),
                5 => {
                    self.step = Step::General;
                    self.row = 0;
                }
                _ => self.advance(),
            },
            Step::Review => {
                if self.row == 0 {
                    self.step = Step::Filters;
                    self.row = 0;
                }
            }
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("Не удалось включить raw mode")?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(mut terminal) => {
                if let Err(error) = terminal.hide_cursor() {
                    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
                    let _ = disable_raw_mode();
                    return Err(error.into());
                }
                Ok(Self { terminal })
            }
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                Err(error.into())
            }
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn run_setup(initial: Option<SearchConfig>) -> Result<Option<SearchConfig>> {
    let mut session = TerminalSession::enter()?;
    let mut app = initial.map(App::from_config).unwrap_or_default();
    loop {
        session.terminal.draw(|frame| draw(frame, &app))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if let Some(modal) = app.modal.take() {
            match modal {
                Modal::Edit(mut editor) => match key.code {
                    KeyCode::Esc => {}
                    KeyCode::Enter => app.save_editor(editor),
                    KeyCode::Left => {
                        editor.cursor = editor.cursor.saturating_sub(1);
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::Right => {
                        editor.cursor = (editor.cursor + 1).min(editor.value.chars().count());
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::Home => {
                        editor.cursor = 0;
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::End => {
                        editor.cursor = editor.value.chars().count();
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::Backspace => {
                        if editor.cursor > 0 {
                            let mut chars = editor.value.chars().collect::<Vec<_>>();
                            chars.remove(editor.cursor - 1);
                            editor.value = chars.into_iter().collect();
                            editor.cursor -= 1;
                        }
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::Delete => {
                        let mut chars = editor.value.chars().collect::<Vec<_>>();
                        if editor.cursor < chars.len() {
                            chars.remove(editor.cursor);
                        }
                        editor.value = chars.into_iter().collect();
                        app.modal = Some(Modal::Edit(editor));
                    }
                    KeyCode::Char(c) if !c.is_control() => {
                        let mut chars = editor.value.chars().collect::<Vec<_>>();
                        chars.insert(editor.cursor, c);
                        editor.value = chars.into_iter().collect();
                        editor.cursor += 1;
                        app.modal = Some(Modal::Edit(editor));
                    }
                    _ => app.modal = Some(Modal::Edit(editor)),
                },
                Modal::Choice(mut choice) => {
                    let len = choice.selected.len();
                    match key.code {
                        KeyCode::Esc => {}
                        KeyCode::Up => {
                            choice.cursor = (choice.cursor + len - 1) % len;
                            app.modal = Some(Modal::Choice(choice));
                        }
                        KeyCode::Down => {
                            choice.cursor = (choice.cursor + 1) % len;
                            app.modal = Some(Modal::Choice(choice));
                        }
                        KeyCode::Char(' ') => {
                            App::toggle_choice(&mut choice);
                            app.modal = Some(Modal::Choice(choice));
                        }
                        KeyCode::Enter => app.save_choice(choice),
                        _ => app.modal = Some(Modal::Choice(choice)),
                    }
                }
            }
            continue;
        }

        match key.code {
            KeyCode::Esc => {
                if !app.back() {
                    return Ok(None);
                }
            }
            KeyCode::Up => app.move_row(-1),
            KeyCode::Down => app.move_row(1),
            KeyCode::Char(' ') => match app.step {
                Step::General if app.row == 2 => app.bio = !app.bio,
                Step::Profiles => app.choose_profile(),
                Step::Review if app.row == 1 => return app.selection().map(Some),
                Step::General | Step::Filters | Step::Review => app.activate(),
            },
            KeyCode::Enter => {
                if app.step == Step::Review && app.row == 1 {
                    return app.selection().map(Some);
                }
                app.activate();
            }
            _ => {}
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let compact = area.width < 86 || area.height < 24;
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let (step_number, step_name, subtitle) = match app.step {
        Step::Profiles => (
            "01",
            "ПРОФИЛЬ",
            "Выберите готовый сценарий или начните с чистых критериев",
        ),
        Step::General => (
            "02",
            "ОБЩЕЕ",
            "Укажите систему, лимит результатов и правила исключения",
        ),
        Step::Filters => (
            "03",
            "ФИЛЬТРЫ",
            "Настройте физические параметры подходящих миров",
        ),
        Step::Review => (
            "04",
            "ПРОВЕРКА",
            "Убедитесь, что критерии поиска заданы правильно",
        ),
    };

    let header = vec![
        Line::from(vec![
            Span::styled(
                " ED ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  EXPLORER  ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" / PLANET SEARCH", Style::default().fg(Color::LightCyan)),
            Span::raw("                                      "),
            Span::styled(
                format!("ШАГ {step_number}/04  {step_name}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(subtitle, Style::default().fg(Color::Gray))),
    ];
    frame.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        root[0],
    );

    match app.step {
        Step::Profiles => draw_profiles(frame, root[1], app, compact),
        Step::General => draw_general(frame, root[1], app),
        Step::Filters => draw_filters(frame, root[1], app),
        Step::Review => draw_review(frame, root[1], app),
    }

    let footer = match app.modal.as_ref() {
        Some(Modal::Edit(_)) => {
            "←/→ курсор   Home/End начало/конец   Enter сохранить   Esc отменить"
        }
        Some(Modal::Choice(Choice {
            field: ChoiceField::Surface,
            ..
        })) => "↑/↓ выбор   Space отметить (можно несколько)   Enter применить   Esc отменить",
        Some(Modal::Choice(_)) => "↑/↓ выбор   Space отметить   Enter применить   Esc отменить",
        None => {
            "↑/↓ перемещение   Enter открыть / продолжить   Space переключить / выбрать   Esc назад"
        }
    };
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
        root[2],
    );

    if let Some(error) = &app.error {
        let popup = centered(72, 25, area);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(error.as_str())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" ПРОВЕРЬТЕ ПАРАМЕТРЫ ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                ),
            popup,
        );
    }
    if let Some(modal) = &app.modal {
        match modal {
            Modal::Edit(editor) => draw_editor(frame, area, editor),
            Modal::Choice(choice) => draw_choice(frame, area, choice),
        }
    }
}

fn draw_profiles(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, compact: bool) {
    let panes = if compact {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area)
    };
    let items = PRESETS
        .iter()
        .enumerate()
        .map(|(index, preset)| {
            let marker = if index == app.profile { "▸ " } else { "  " };
            let line = format!("{marker}{:02}  {}", preset.number, preset.name);
            let style = if index == app.profile {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .chain(std::iter::once(ListItem::new(Line::from(Span::styled(
            format!(
                "{}✦  Своя настройка",
                if app.profile == PRESETS.len() {
                    "▸ "
                } else {
                    "  "
                }
            ),
            if app.profile == PRESETS.len() {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Magenta)
            },
        )))))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.profile));
    frame.render_stateful_widget(
        List::new(items).block(panel(" ВЫБЕРИТЕ ПРОФИЛЬ ", Color::Blue)),
        panes[0],
        &mut state,
    );

    let mut info = Vec::new();
    if app.profile < PRESETS.len() {
        let p = &PRESETS[app.profile];
        info.extend(preset_lines(p));
        info.push(Line::from(""));
        info.push(Line::from(Span::styled(
            format!("Справочно: до {} дорогих видов", p.expensive_species),
            Style::default().fg(Color::Magenta),
        )));
        info.push(Line::from(Span::styled(
            format!("Потенциал: {} млн CR", p.potential_value_cr / 1_000_000),
            Style::default().fg(Color::Magenta),
        )));
    } else {
        info.push(Line::from(Span::styled(
            "Начните с нейтральных значений.",
            Style::default().fg(Color::White),
        )));
        info.push(Line::from(
            "Все критерии можно настроить вручную на следующих шагах.",
        ));
    }
    frame.render_widget(
        Paragraph::new(info)
            .wrap(Wrap { trim: true })
            .block(panel(" ПАРАМЕТРЫ ПРОФИЛЯ ", Color::Magenta)),
        panes[1],
    );
}

fn preset_lines(p: &SearchPreset) -> Vec<Line<'static>> {
    vec![
        kv(
            "Тип поверхности",
            p.surface_types
                .iter()
                .map(|surface| surface.spansh_value())
                .collect::<Vec<_>>()
                .join(" / "),
            Color::LightCyan,
        ),
        kv(
            "Атмосфера",
            p.atmosphere.spansh_value().to_owned(),
            Color::White,
        ),
        kv(
            "Температура",
            format!("{:.0}–{:.0} K", p.temperature.min, p.temperature.max),
            Color::Yellow,
        ),
        kv(
            "Давление",
            format!("{:.3}–{:.3} atm", p.pressure.min, p.pressure.max),
            Color::Yellow,
        ),
        kv("Гравитация", "0.1–1.0 g".into(), Color::Yellow),
        kv("Вулканизм", "не фильтруется".into(), Color::Green),
    ]
}

fn draw_general(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let rows = vec![
        menu_line(
            0,
            app.row,
            "Центральная система",
            &app.system,
            "Enter — изменить",
        ),
        menu_line(
            1,
            app.row,
            "Количество результатов",
            &app.count,
            "Enter — изменить",
        ),
        Line::from(vec![
            Span::styled(
                if app.row == 2 { "▸ " } else { "  " },
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                "Исключать планеты с биосигналами",
                Style::default().fg(Color::White),
            ),
            Span::raw("   "),
            Span::styled(
                if app.bio { "[ ВКЛ ]" } else { "[ ВЫКЛ]" },
                Style::default()
                    .fg(if app.bio {
                        Color::Green
                    } else {
                        Color::DarkGray
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        action_line(3, app.row, "Продолжить к фильтрам  →"),
    ];
    frame.render_widget(
        Paragraph::new(rows).block(panel(" ПАРАМЕТРЫ ЗАПУСКА ", Color::Cyan)),
        area,
    );
}

fn draw_filters(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let surface = app.surfaces.join("  OR  ");
    let rows = vec![
        menu_line(0, app.row, "Тип поверхности", &surface, "Enter — выбрать"),
        menu_line(1, app.row, "Атмосфера", &app.atmosphere, "Enter — выбрать"),
        menu_line(
            2,
            app.row,
            "Температура поверхности",
            &format!("{} K", app.temperature),
            "Enter — диапазон",
        ),
        menu_line(
            3,
            app.row,
            "Гравитация",
            &format!("{} g", app.gravity),
            "Enter — диапазон",
        ),
        menu_line(
            4,
            app.row,
            "Давление",
            &format!("{} atm", app.pressure),
            "Enter — диапазон",
        ),
        action_line(5, app.row, "← Назад к общим параметрам"),
        action_line(6, app.row, "Проверить критерии  →"),
        Line::from(""),
        Line::from(Span::styled(
            "Вулканизм не используется как фильтр.",
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            "Несколько типов поверхности объединяются через OR.",
            Style::default().fg(Color::Gray),
        )),
    ];
    frame.render_widget(
        Paragraph::new(rows).block(panel(" КРИТЕРИИ ПЛАНЕТЫ ", Color::Magenta)),
        area,
    );
}

fn draw_review(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let result = app.selection();
    let mut rows = vec![
        kv("Центральная система", app.system.clone(), Color::LightCyan),
        kv(
            "Профиль",
            if app.profile < PRESETS.len() {
                PRESETS[app.profile].name.to_owned()
            } else {
                "Своя настройка".into()
            },
            Color::White,
        ),
        kv("Тип поверхности", app.surfaces.join(" OR "), Color::White),
        kv("Атмосфера", app.atmosphere.clone(), Color::White),
        kv(
            "Температура",
            format!("{} K", app.temperature),
            Color::Yellow,
        ),
        kv("Гравитация", format!("{} g", app.gravity), Color::Yellow),
        kv("Давление", format!("{} atm", app.pressure), Color::Yellow),
        kv("Количество", app.count.clone(), Color::White),
        kv(
            "Исключать биосигналы",
            if app.bio {
                "Да".into()
            } else {
                "Нет".into()
            },
            if app.bio { Color::Green } else { Color::Gray },
        ),
        kv("Вулканизм", "не фильтруется".into(), Color::Green),
        Line::from(""),
        action_line(0, app.row, "← Вернуться к фильтрам"),
        action_line(1, app.row, "▶ Запустить поиск"),
    ];
    if let Err(error) = result {
        rows.push(Line::from(""));
        rows.push(Line::from(Span::styled(
            error.to_string(),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(
        Paragraph::new(rows).block(panel(" ПРОВЕРКА ПЕРЕД ЗАПУСКОМ ", Color::Green)),
        area,
    );
}

fn menu_line(index: usize, selected: usize, label: &str, value: &str, hint: &str) -> Line<'static> {
    let active = index == selected;
    Line::from(vec![
        Span::styled(
            if active { "▸ " } else { "  " },
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{label:<30}"),
            Style::default()
                .fg(if active { Color::White } else { Color::Gray })
                .add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            value.to_owned(),
            Style::default().fg(if active {
                Color::Yellow
            } else {
                Color::LightCyan
            }),
        ),
        Span::styled(format!("   {hint}"), Style::default().fg(Color::DarkGray)),
    ])
}

fn action_line(index: usize, selected: usize, label: &str) -> Line<'static> {
    let active = index == selected;
    Line::from(Span::styled(
        format!("{}{}", if active { "▸ " } else { "  " }, label),
        Style::default()
            .fg(if active { Color::Green } else { Color::Gray })
            .add_modifier(if active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    ))
}

fn kv(label: &str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<25}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn panel(title: &'static str, color: Color) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
}

fn draw_editor(frame: &mut ratatui::Frame<'_>, area: Rect, editor: &Editor) {
    let popup = centered(72, 34, area);
    frame.render_widget(Clear, popup);
    let label = match editor.field {
        EditField::System => "Название центральной системы",
        EditField::Count => "Количество результатов",
        EditField::Temperature => "Диапазон температуры (min-max, K)",
        EditField::Gravity => "Диапазон гравитации (min-max, g)",
        EditField::Pressure => "Диапазон давления (min-max, atm)",
    };
    let before = editor.value.chars().take(editor.cursor).collect::<String>();
    let after = editor.value.chars().skip(editor.cursor).collect::<String>();
    let content = format!(
        "{label}\n\n{before}▏{after}\n\nСтрелки ←/→ перемещают курсор. Enter сохраняет значение."
    );
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(panel(" РЕДАКТИРОВАНИЕ ", Color::Yellow)),
        popup,
    );
}

fn draw_choice(frame: &mut ratatui::Frame<'_>, area: Rect, choice: &Choice) {
    let popup = centered(68, 74, area);
    frame.render_widget(Clear, popup);
    let options: &[&str] = match choice.field {
        ChoiceField::Surface => &SURFACES,
        ChoiceField::Atmosphere => &ATMOSPHERES,
    };
    let items = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let selected = choice.selected[index];
            let cursor = index == choice.cursor;
            let marker = if cursor { "▸" } else { " " };
            let check = if selected { "[✓]" } else { "[ ]" };
            let style = if cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker} {check} {option}"),
                style,
            )))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(choice.cursor));
    frame.render_stateful_widget(
        List::new(items).block(panel(
            match choice.field {
                ChoiceField::Surface => " ТИПЫ ПОВЕРХНОСТИ ",
                ChoiceField::Atmosphere => " АТМОСФЕРА ",
            },
            Color::Cyan,
        )),
        popup,
        &mut state,
    );
}

fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_navigation_updates_the_profile_and_allows_custom_setup() {
        let mut app = App::default();

        app.move_row(2);
        assert_eq!(app.profile, 2);
        app.choose_profile();
        assert_eq!(app.surfaces, vec!["Rocky body", "High metal content world"]);
        assert_eq!(app.atmosphere, "Water");

        let mut app = App::default();
        app.move_row(PRESETS.len() as isize);
        assert_eq!(app.profile, PRESETS.len());
        app.choose_profile();
        assert_eq!(app.step, Step::General);
        assert_eq!(app.surfaces, vec!["any"]);
        assert_eq!(app.atmosphere, "any");
    }

    #[test]
    fn cli_configuration_prefills_the_wizard_without_changing_criteria() {
        let mut criteria = SearchCriteria::new(
            Filter::one_of(vec![SurfaceType::RockyBody]),
            Filter::one_of(vec![AtmosphereType::Water]),
            NumericRange {
                min: 395.0,
                max: 450.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.040,
                max: 0.063,
            },
            6,
        )
        .unwrap()
        .with_bio_signal_exclusion(true);
        criteria.surface = Filter::one_of(vec![
            SurfaceType::RockyBody,
            SurfaceType::HighMetalContentWorld,
        ]);
        let config = SearchConfig::new("Sol", criteria).unwrap();

        let app = App::from_config(config.clone());
        assert_eq!(app.step, Step::General);
        assert_eq!(app.profile, PRESETS.len());
        assert_eq!(app.selection().unwrap(), config);
    }

    #[test]
    fn returning_to_profiles_keeps_the_current_profile_cursor() {
        let mut app = App::default();
        app.move_row(4);
        app.choose_profile();
        assert!(app.back());
        assert_eq!(app.step, Step::Profiles);
        assert_eq!(app.row, 4);
        assert_eq!(app.profile, 4);
    }

    #[test]
    fn selected_preset_preserves_profile_three_or_surface_types_and_gravity() {
        let mut app = App {
            profile: 2,
            ..App::default()
        };
        app.choose_profile();
        let config = app.selection().unwrap();
        assert_eq!(
            config.criteria.surface,
            Filter::one_of(vec![
                SurfaceType::RockyBody,
                SurfaceType::HighMetalContentWorld,
            ])
        );
        assert_eq!(config.criteria.gravity, NumericRange { min: 0.1, max: 1.0 });
    }

    #[test]
    fn profile_one_preserves_temperature_pressure_and_bio_switch() {
        let mut app = App {
            profile: 0,
            bio: true,
            ..App::default()
        };
        app.choose_profile();
        let config = app.selection().unwrap();
        assert_eq!(
            config.criteria.temperature,
            NumericRange {
                min: 190.0,
                max: 195.0
            }
        );
        assert_eq!(
            config.criteria.pressure,
            NumericRange {
                min: 0.042,
                max: 0.237
            }
        );
        assert!(config.criteria.exclude_with_bio_signals);
    }

    #[test]
    fn surface_choice_uses_or_and_any_is_exclusive() {
        let mut choice = Choice {
            field: ChoiceField::Surface,
            cursor: 1,
            selected: vec![true, false, false, false, false],
        };
        App::toggle_choice(&mut choice);
        assert_eq!(choice.selected, vec![false, true, false, false, false]);
        choice.cursor = 2;
        App::toggle_choice(&mut choice);
        assert_eq!(choice.selected, vec![false, true, true, false, false]);
        choice.cursor = 0;
        App::toggle_choice(&mut choice);
        assert_eq!(choice.selected, vec![true, false, false, false, false]);
    }

    #[test]
    fn rejects_invalid_ranges_and_zero_count() {
        let mut app = App {
            count: "0".into(),
            ..App::default()
        };
        assert!(app.selection().is_err());
        app.count = "5".into();
        app.temperature = "200-100".into();
        assert!(app.selection().is_err());
    }
}
