use std::io::{self, Stdout};

use anyhow::{Context, Result, anyhow};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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
    application::{config::SearchConfig, presets::{PRESETS, SearchPreset}},
    cli::{parse_range_f32, parse_range_i32},
    domain::criteria::{NumericRange, SearchCriteria},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Presets,
    Manual,
    Confirm,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    System,
    Count,
    Bio,
    Planet,
    Atmosphere,
    Temperature,
    Gravity,
    Pressure,
}

struct App {
    screen: Screen,
    manual_mode: bool,
    selected: usize,
    focus: usize,
    system: String,
    count: String,
    bio: bool,
    planet: String,
    atmosphere: String,
    temperature: String,
    gravity: String,
    pressure: String,
    error: Option<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Presets,
            manual_mode: false,
            selected: 0,
            focus: 0,
            system: "Sol".into(),
            count: "5".into(),
            bio: false,
            planet: "any".into(),
            atmosphere: "any".into(),
            temperature: "0-5000".into(),
            gravity: "0.1-1.0".into(),
            pressure: "0-10".into(),
            error: None,
        }
    }
}
impl App {
    fn fields(&self) -> &'static [Field] {
        match self.screen {
            Screen::Presets => &[Field::System, Field::Count, Field::Bio],
            Screen::Manual => &[
                Field::System,
                Field::Planet,
                Field::Atmosphere,
                Field::Temperature,
                Field::Gravity,
                Field::Pressure,
                Field::Count,
                Field::Bio,
            ],
            Screen::Confirm => &[],
        }
    }
    fn focused(&self) -> Option<Field> {
        self.fields().get(self.focus).copied()
    }
    fn move_focus(&mut self, d: isize) {
        let n = self.fields().len();
        if n > 0 {
            self.focus = (self.focus as isize + d).rem_euclid(n as isize) as usize;
        }
        self.error = None;
    }
    fn edit(&mut self, code: KeyCode) {
        let field = match self.focused() {
            Some(v) => v,
            None => return,
        };
        if field == Field::Bio {
            if matches!(code, KeyCode::Char(' ') | KeyCode::Enter) {
                self.bio = !self.bio;
            }
            return;
        }
        let value = match field {
            Field::System => &mut self.system,
            Field::Count => &mut self.count,
            Field::Planet => &mut self.planet,
            Field::Atmosphere => &mut self.atmosphere,
            Field::Temperature => &mut self.temperature,
            Field::Gravity => &mut self.gravity,
            Field::Pressure => &mut self.pressure,
            Field::Bio => return,
        };
        match code {
            KeyCode::Char(c) => value.push(c),
            KeyCode::Backspace => {
                value.pop();
            }
            KeyCode::Delete => value.clear(),
            _ => {}
        }
        self.error = None;
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
        let criteria = if self.manual_mode {
            let (a, b) =
                parse_range_i32(&self.temperature).context("Температура: диапазон min-max в K")?;
            let (c, d) =
                parse_range_f32(&self.gravity).context("Гравитация: диапазон min-max в g")?;
            let (e, f) =
                parse_range_f32(&self.pressure).context("Давление: диапазон min-max в atm")?;
            let surface_types = self
                .planet
                .split('|')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if surface_types.is_empty() {
                return Err(anyhow!(
                    "Укажите тип поверхности или явно задайте any для отключения фильтра"
                ));
            }
            let primary_surface = surface_types.first().copied().unwrap_or("any");
            let mut c = SearchCriteria::from_cli(
                primary_surface,
                &self.atmosphere,
                NumericRange::new(a.into(), b.into())
                    .ok_or_else(|| anyhow!("Некорректная температура"))?,
                NumericRange::new(c.into(), d.into())
                    .ok_or_else(|| anyhow!("Некорректная гравитация"))?,
                NumericRange::new(e.into(), f.into())
                    .ok_or_else(|| anyhow!("Некорректное давление"))?,
                count,
            )?;
            if self.planet.contains('|') {
                c.planet_types = surface_types.iter().map(|s| (*s).to_owned()).collect();
                c.planet_type = c
                    .planet_types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "any".into());
            }
            c.with_bio_signal_exclusion(self.bio)
        } else {
            PRESETS[self.selected]
                .criteria(5, self.bio)?
                .with_count(count)?
        };
        SearchConfig::new(self.system.clone(), criteria).map_err(Into::into)
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}
impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("Не удалось включить raw mode")?;
        let mut out = io::stdout();
        if let Err(e) = execute!(out, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        match Terminal::new(CrosstermBackend::new(out)) {
            Ok(mut terminal) => {
                if let Err(error) = terminal.hide_cursor() {
                    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
                    let _ = disable_raw_mode();
                    return Err(error.into());
                }
                Ok(Self { terminal })
            }
            Err(e) => {
                let mut out = io::stdout();
                let _ = execute!(out, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                Err(e.into())
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

pub fn run_tui() -> Result<Option<SearchConfig>> {
    super::setup::run_setup()
}

fn draw(f: &mut ratatui::Frame<'_>, app: &App) {
    let area = f.area();
    let compact = area.width < 92 || area.height < 28;
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Line::from(vec![
        Span::styled(" ED ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("  EXPLORER  ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" / ", Style::default().fg(Color::DarkGray)),
        Span::styled("ПОДБОР ПЛАНЕТ", Style::default().fg(Color::LightCyan)),
        Span::styled("                                      ", Style::default()),
        Span::styled(if app.screen == Screen::Manual { "РУЧНОЙ РЕЖИМ" } else { "ПРОФИЛИ ПОИСКА" },
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(
        Paragraph::new(header).block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))),
        root[0],
    );

    let main = if compact {
        Layout::default().direction(Direction::Vertical)
            .constraints([Constraint::Percentage(43), Constraint::Percentage(57)])
            .split(root[1])
    } else {
        Layout::default().direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(root[1])
    };

    let profile_items = PRESETS.iter().enumerate().map(|(i, p)| {
        let selected = i == app.selected;
        let marker = if selected { "▸ " } else { "  " };
        let label = format!("{marker}{:02}  {}", p.number, p.name);
        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        ListItem::new(Line::from(Span::styled(label, style)))
    }).collect::<Vec<_>>();
    let mut profile_state = ListState::default();
    profile_state.select(Some(app.selected));
    f.render_stateful_widget(
        List::new(profile_items).highlight_symbol(" ").block(
            Block::default().title(" 01 / ПРОФИЛЬ  [ ↑ / ↓ ] ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
        ),
        main[0],
        &mut profile_state,
    );

    let detail_and_form = if compact {
        Layout::default().direction(Direction::Vertical)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
            .split(main[1])
    } else {
        Layout::default().direction(Direction::Vertical)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
            .split(main[1])
    };
    draw_details(f, detail_and_form[0], &PRESETS[app.selected]);
    draw_form(f, detail_and_form[1], app);

    let footer = Line::from(vec![
        Span::styled(" TAB ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" поле  "),
        Span::styled(" ↑ / ↓ ", Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD)),
        Span::raw(" профиль  "),
        Span::styled(" F2 ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw(" ручной режим  "),
        Span::styled(" F5 ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" далее  "),
        Span::styled(" ESC ", Style::default().fg(Color::Black).bg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::raw(" назад / выход"),
    ]);
    f.render_widget(Paragraph::new(footer).alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))),
        root[2]);

    if let Some(error) = &app.error {
        let popup = centered(72, 24, area);
        f.render_widget(Clear, popup);
        f.render_widget(
            Paragraph::new(error.as_str()).wrap(Wrap { trim: true })
                .block(Block::default().title(" ! ПРОВЕРЬТЕ ПАРАМЕТРЫ ")
                    .borders(Borders::ALL).border_style(Style::default().fg(Color::Red))),
            popup,
        );
    }
    if app.screen == Screen::Confirm {
        let popup = centered(74, 58, area);
        f.render_widget(Clear, popup);
        let p = &PRESETS[app.selected];
        let summary = format!(
            "ЦЕНТРАЛЬНАЯ СИСТЕМА   {}\nПРОФИЛЬ               {}\n\nПОВЕРХНОСТЬ           {}\nАТМОСФЕРА             {}\nТЕМПЕРАТУРА           {} K\nГРАВИТАЦИЯ            {} g\nДАВЛЕНИЕ              {} atm\nЛИМИТ                 {} тел\nИСКЛЮЧАТЬ БИОСИГНАЛЫ  {}\n\nEnter — запустить поиск    Esc — вернуться",
            app.system, p.name, app.planet, app.atmosphere, app.temperature,
            app.gravity, app.pressure, app.count, if app.bio { "да" } else { "нет" }
        );
        f.render_widget(Paragraph::new(summary).wrap(Wrap { trim: true })
            .block(Block::default().title(" 03 / ПРОВЕРКА ПОИСКА ")
                .borders(Borders::ALL).border_style(Style::default().fg(Color::Green))),
            popup);
    }
}

fn draw_details(f: &mut ratatui::Frame<'_>, area: Rect, p: &SearchPreset) {
    let rows = vec![
        Line::from(vec![Span::styled("ТИП МИРА      ", Style::default().fg(Color::DarkGray)), Span::styled(p.surface_types.join("  OR  "), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("АТМОСФЕРА     ", Style::default().fg(Color::DarkGray)), Span::styled(p.atmosphere, Style::default().fg(Color::White))]),
        Line::from(vec![Span::styled("ТЕМПЕРАТУРА   ", Style::default().fg(Color::DarkGray)), Span::styled(format!("{:.0}–{:.0} K", p.temperature.min, p.temperature.max), Style::default().fg(Color::Yellow))]),
        Line::from(vec![Span::styled("ДАВЛЕНИЕ      ", Style::default().fg(Color::DarkGray)), Span::styled(format!("{:.3}–{:.3} atm", p.pressure.min, p.pressure.max), Style::default().fg(Color::Yellow))]),
        Line::from(vec![Span::styled("ГРАВИТАЦИЯ    ", Style::default().fg(Color::DarkGray)), Span::styled("0.1–1.0 g", Style::default().fg(Color::Yellow))]),
        Line::from(vec![Span::styled("ВУЛКАНИЗМ     ", Style::default().fg(Color::DarkGray)), Span::styled("не фильтруется", Style::default().fg(Color::Green))]),
        Line::from(vec![Span::styled("СПРАВОЧНО     ", Style::default().fg(Color::DarkGray)), Span::styled(format!("до {} дорогих видов · {} млн CR", p.expensive_species, p.potential_value_cr / 1_000_000), Style::default().fg(Color::Magenta))]),
    ];
    f.render_widget(Paragraph::new(rows).wrap(Wrap { trim: true }).block(
        Block::default().title(" 02 / УСЛОВИЯ ПРОФИЛЯ ")
            .borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta))),
        area);
}

fn draw_form(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let mut rows = Vec::new();
    let mut add_field = |label: &str, value: &str, field: Field| {
        let active = app.focused() == Some(field);
        let label_style = Style::default().fg(if active { Color::Black } else { Color::DarkGray });
        let value_style = if active {
            Style::default().fg(Color::Black).bg(Color::LightYellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        rows.push(Line::from(vec![
            Span::styled(format!(" {:<20} ", label), label_style.bg(if active { Color::LightYellow } else { Color::Reset })),
            Span::styled(if value.is_empty() { " <пусто> " } else { value }, value_style),
            if active { Span::styled("  ◀ ввод", Style::default().fg(Color::Yellow)) } else { Span::raw("") },
        ]));
    };

    add_field("Центральная система", &app.system, Field::System);
    if app.screen == Screen::Manual {
        add_field("Тип поверхности", &app.planet, Field::Planet);
        add_field("Атмосфера", &app.atmosphere, Field::Atmosphere);
        add_field("Температура, K", &app.temperature, Field::Temperature);
        add_field("Гравитация, g", &app.gravity, Field::Gravity);
        add_field("Давление, atm", &app.pressure, Field::Pressure);
    }
    add_field("Количество результатов", &app.count, Field::Count);
    rows.push(Line::from(""));
    let bio_style = if app.bio { Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Yellow) };
    rows.push(Line::from(vec![
        Span::styled(if app.bio { " [✓ ВКЛ] " } else { " [○ ВЫКЛ] " }, bio_style),
        Span::styled("Исключать тела с биосигналами", Style::default().fg(Color::White)),
    ]));
    rows.push(Line::from(Span::styled(
        if app.screen == Screen::Manual { "Типы через | ; any отключает фильтр" } else { "Tab — переход между полями · ввод добавляет символы" },
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(rows).wrap(Wrap { trim: true }).block(
        Block::default().title(if app.screen == Screen::Manual { " 03 / РУЧНАЯ НАСТРОЙКА " } else { " 03 / ПАРАМЕТРЫ ЗАПУСКА " })
            .borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan))),
        area);
}
fn centered(x: u16, y: u16, a: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - y) / 2),
            Constraint::Percentage(y),
            Constraint::Percentage((100 - y) / 2),
        ])
        .split(a);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - x) / 2),
            Constraint::Percentage(x),
            Constraint::Percentage((100 - x) / 2),
        ])
        .split(v[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_three_keeps_or_surface_types() {
        let a = App {
            selected: 2,
            ..App::default()
        };
        let s = a.selection().unwrap();
        assert_eq!(
            s.criteria.planet_types,
            vec!["Rocky body", "High metal content world"]
        );
        assert_eq!(s.criteria.gravity, NumericRange { min: 0.1, max: 1.0 });
    }
    #[test]
    fn zero_count_is_rejected() {
        let a = App {
            count: "0".into(),
            ..App::default()
        };
        assert!(a.selection().is_err());
    }
    #[test]
    fn bio_exclusion_is_independent() {
        let a = App {
            bio: true,
            ..App::default()
        };
        assert!(a.selection().unwrap().criteria.exclude_with_bio_signals);
    }
    #[test]
    fn manual_mode_accepts_any_filters() {
        let a = App {
            screen: Screen::Manual,
            manual_mode: true,
            ..App::default()
        };
        assert!(a.selection().is_ok());
    }

    fn sortable_body(
        name: &str,
        distance: Option<f64>,
        temperature: Option<f64>,
    ) -> crate::domain::body::Body {
        crate::domain::body::Body {
            id64: None,
            name: name.to_owned(),
            system_id64: None,
            system_name: None,
            subtype: None,
            distance_ly: distance,
            is_landable: None,
            is_mapped: None,
            last_updated_at: None,
            surface_temperature: temperature,
            surface_pressure: None,
            gravity: None,
            atmosphere: None,
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn results_sort_by_selected_numeric_field_and_keep_missing_values_last() {
        let mut bodies = vec![
            sortable_body("Unknown", None, None),
            sortable_body("Far", Some(20.0), Some(180.0)),
            sortable_body("Near", Some(5.0), Some(220.0)),
        ];

        sort_bodies(&mut bodies, 0);
        assert_eq!(
            bodies.iter().map(|body| body.name.as_str()).collect::<Vec<_>>(),
            vec!["Near", "Far", "Unknown"]
        );

        sort_bodies(&mut bodies, 1);
        assert_eq!(
            bodies.iter().map(|body| body.name.as_str()).collect::<Vec<_>>(),
            vec!["Far", "Near", "Unknown"]
        );
    }
}

/// Выполняет асинхронный поиск, не блокируя полноэкранный интерфейс.
pub async fn run_search_ui<F>(future: F) -> Result<()>
where
    F: std::future::Future<Output = Result<Vec<crate::domain::body::Body>>> + Send + 'static,
{
    use std::time::Duration;

    let mut session = TerminalSession::enter()?;
    let task = tokio::spawn(future);

    let outcome = loop {
        if task.is_finished() {
            break match task.await {
                Ok(result) => result,
                Err(error) => Err(anyhow!("Задача поиска аварийно завершилась: {error}")),
            };
        }

        session.terminal.draw(draw_searching)?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && (key.code == KeyCode::Esc
                        || (key.code == KeyCode::Char('q')
                            && key.modifiers.contains(KeyModifiers::CONTROL)))
                {
                    task.abort();
                    return Ok(());
                }
            }
        }
    };

    match outcome {
        Ok(bodies) => show_results_in_session(&mut session, &bodies),
        Err(error) => show_error_in_session(&mut session, &error.to_string()),
    }
}

fn show_error_in_session(session: &mut TerminalSession, message: &str) -> Result<()> {
    loop {
        session.terminal.draw(|frame| {
            let panel = centered(75, 40, frame.area());
            frame.render_widget(
                Paragraph::new(format!("Поиск не выполнен\n\n{message}\n\nEnter / Esc — вернуться и закрыть"))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(Block::default().title(" ОШИБКА ПОИСКА ").borders(Borders::ALL).border_style(Style::default().fg(Color::Red))),
                panel,
            );
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
                return Ok(());
            }
        }
    }
}

fn draw_searching(frame: &mut ratatui::Frame<'_>) {
    let area = frame.area();
    let panel = centered(70, 35, area);
    frame.render_widget(
        Paragraph::new(
            "Выполняется поиск планет\n\nРазрешение центральной системы и загрузка результатов Spansh.\n\nИнтерфейс остаётся активным. Esc — отменить поиск.",
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" ED: PLANET SEARCH ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        panel,
    );
}

fn show_results_in_session(
    session: &mut TerminalSession,
    bodies: &[crate::domain::body::Body],
) -> Result<()> {
    use std::time::Duration;

    let mut sorted_bodies = bodies.to_vec();
    let mut sort_mode = 0usize;
    sort_bodies(&mut sorted_bodies, sort_mode);
    let mut selected = 0usize;
    let mut detail_scroll = 0u16;
    loop {
        session.terminal.draw(|frame| {
            draw_results(frame, &sorted_bodies, selected, detail_scroll, sort_mode)
        })?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
            KeyCode::Up if !bodies.is_empty() => selected = selected.saturating_sub(1),
            KeyCode::Down if !bodies.is_empty() => selected = (selected + 1).min(bodies.len() - 1),
            KeyCode::Home => selected = 0,
            KeyCode::End if !bodies.is_empty() => selected = bodies.len() - 1,
            KeyCode::PageUp => detail_scroll = detail_scroll.saturating_sub(8),
            KeyCode::PageDown => detail_scroll = detail_scroll.saturating_add(8),
            KeyCode::Char('s') => {
                sort_mode = (sort_mode + 1) % 5;
                sort_bodies(&mut sorted_bodies, sort_mode);
                selected = 0;
                detail_scroll = 0;
            }
            _ => {}
        }
    }
}

fn sort_bodies(bodies: &mut [crate::domain::body::Body], mode: usize) {
    use std::cmp::Ordering;

    fn compare_optional(left: Option<f64>, right: Option<f64>) -> Ordering {
        match (left, right) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    bodies.sort_by(|left, right| {
        match mode {
            0 => compare_optional(left.distance_ly, right.distance_ly),
            1 => compare_optional(left.surface_temperature, right.surface_temperature),
            2 => compare_optional(left.gravity, right.gravity),
            3 => compare_optional(left.surface_pressure, right.surface_pressure),
            _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        }
    });
}

fn draw_results(
    frame: &mut ratatui::Frame<'_>,
    bodies: &[crate::domain::body::Body],
    selected: usize,
    detail_scroll: u16,
    sort_mode: usize,
) {
    use ratatui::widgets::{List, ListItem};

    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(2)])
        .split(area);

    frame.render_widget(
        Paragraph::new(format!("ED: PLANET SEARCH  /  РЕЗУЛЬТАТЫ: {}", bodies.len()))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan))),
        root[0],
    );

    let wide = area.width >= 100;
    let panes = if wide {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(root[1])
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(root[1])
    };

    let items = bodies.iter().map(|body| {
        let mapped = if body.is_mapped == Some(true) { "  [картографирована]" } else { "" };
        let label = format!("{}{}", body.name, mapped);
        let style = if body.is_mapped == Some(true) {
            Style::default().fg(Color::LightGreen)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(label).style(style)
    }).collect::<Vec<_>>();
    let mut state = ListState::default();
    if !bodies.is_empty() {
        state.select(Some(selected.min(bodies.len() - 1)));
    }
    frame.render_stateful_widget(
        List::new(items).highlight_style(
            Style::default().fg(Color::Black).bg(Color::LightCyan).add_modifier(Modifier::BOLD)
        ).highlight_symbol("▸ ").block(
            Block::default().title(" НАЙДЕННЫЕ ПЛАНЕТЫ ")
                .borders(Borders::ALL).border_style(Style::default().fg(Color::Blue))
        ),
        panes[0],
        &mut state,
    );

    let detail = bodies.get(selected).map_or_else(
        || "Подходящие планеты не найдены.".to_owned(),
        |body| {
            let mut lines = vec![
                format!("Планета: {}", body.name),
                format!("Система: {}", body.system_name.as_deref().unwrap_or("нет данных")),
                format!("Тип: {}", body.subtype.as_deref().unwrap_or("нет данных")),
                format!("Атмосфера: {}", body.atmosphere.as_deref().unwrap_or("нет данных")),
                format!("Температура: {}", body.surface_temperature.map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.2} K"))),
                format!("Давление: {}", body.surface_pressure.map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.4} atm"))),
                format!("Гравитация: {}", body.gravity.map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.3} g"))),
                format!("Расстояние: {}", body.distance_ly.map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.2} св. л."))),
                format!("Биологические сигналы Spansh: {}", body.biological_signal_count.map_or_else(|| "нет данных".to_owned(), |v| v.to_string())),
                format!("Геологические сигналы Spansh: {}", body.geological_signal_count.map_or_else(|| "нет данных".to_owned(), |v| v.to_string())),
                format!("Биологические группы: {}", body.biological_genera.as_ref().filter(|v| !v.is_empty()).map_or_else(|| "не указаны".to_owned(), |v| v.iter().map(|g| crate::presentation::output::russian_group_name(g)).collect::<Vec<_>>().join(", "))),
                format!("Вулканизм: {}", body.volcanism_type.as_deref().unwrap_or("нет данных")),
                String::new(),
            ];
            match crate::presentation::output::exobiology_summary(body) {
                Ok(Some(summary)) => lines.extend(summary.lines().map(str::to_owned)),
                Ok(None) => lines.push("Подходящие виды в справочнике не найдены.".to_owned()),
                Err(error) => lines.push(format!("Ошибка расчёта экзобиологии: {error}")),
            }
            lines.join("\n")
        },
    );
    frame.render_widget(
        Paragraph::new(detail).scroll((detail_scroll, 0)).wrap(Wrap { trim: false }).block(
            Block::default().title(" ПОДРОБНОСТИ ПЛАНЕТЫ ")
                .borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta)),
        ),
        panes[1],
    );
    let sort_label = ["расстояние", "температура", "гравитация", "давление", "название"]
        .get(sort_mode)
        .copied()
        .unwrap_or("расстояние");
    frame.render_widget(
        Paragraph::new(format!(
            "↑/↓ планета   S сортировка: {sort_label}   PgUp/PgDn подробности   Home/End список   Esc/Q выход"
        ))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray))),
        root[2],
    );
}
