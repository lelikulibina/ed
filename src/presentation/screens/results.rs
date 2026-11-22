use std::cmp::Ordering;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use crate::{
    domain::body::Body,
    presentation::{
        app::App,
        text::{ExobiologyView, exobiology_view, format_credits, ui},
        theme,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortMode {
    Distance,
    Temperature,
    Pressure,
    Gravity,
    Name,
}

impl SortMode {
    fn next(self) -> Self {
        match self {
            Self::Distance => Self::Temperature,
            Self::Temperature => Self::Pressure,
            Self::Pressure => Self::Gravity,
            Self::Gravity => Self::Name,
            Self::Name => Self::Distance,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Distance => ui::RESULTS_SORT_DISTANCE,
            Self::Temperature => ui::RESULTS_SORT_TEMPERATURE,
            Self::Pressure => ui::RESULTS_SORT_PRESSURE,
            Self::Gravity => ui::RESULTS_SORT_GRAVITY,
            Self::Name => ui::RESULTS_SORT_NAME,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultsAction {
    BackToSearch,
    CopySystem(String),
}

pub struct ResultsScreen {
    sort_mode: SortMode,
    descending: bool,
    selected_identity: Option<String>,
    detail_scroll: u16,
    status_message: Option<String>,
}

impl Default for ResultsScreen {
    fn default() -> Self {
        Self {
            sort_mode: SortMode::Distance,
            descending: false,
            selected_identity: None,
            detail_scroll: 0,
            status_message: None,
        }
    }
}

impl ResultsScreen {
    pub fn reset(&mut self, bodies: &[Body]) {
        self.sort_mode = SortMode::Distance;
        self.descending = false;
        self.selected_identity = bodies
            .iter()
            .min_by(|left, right| {
                compare_optional(left.distance_ly, right.distance_ly)
                    .then_with(|| left.name.cmp(&right.name))
            })
            .map(body_identity);
        self.detail_scroll = 0;
        self.status_message = None;
    }

    pub fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn handle_key(&mut self, key: KeyEvent, bodies: &[Body]) -> Option<ResultsAction> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        let ordered = self.ordered_bodies(bodies);
        let selected = self.selected_index(&ordered);
        match key.code {
            KeyCode::Up if !ordered.is_empty() => {
                self.select(ordered[selected.saturating_sub(1)]);
            }
            KeyCode::Down if !ordered.is_empty() => {
                self.select(ordered[(selected + 1).min(ordered.len() - 1)]);
            }
            KeyCode::PageUp => self.detail_scroll = self.detail_scroll.saturating_sub(6),
            KeyCode::PageDown => self.detail_scroll = self.detail_scroll.saturating_add(6),
            KeyCode::Char('s') => {
                let selected_identity = self.selected_identity.clone();
                self.sort_mode = self.sort_mode.next();
                self.selected_identity = selected_identity;
            }
            KeyCode::Char('S') => self.descending = !self.descending,
            KeyCode::Char('c') => {
                return ordered
                    .get(selected)
                    .and_then(|body| body.system_name.clone())
                    .map(ResultsAction::CopySystem);
            }
            KeyCode::Char('e' | 'E' | 'n' | 'N') => {
                return Some(ResultsAction::BackToSearch);
            }
            _ => {}
        }
        None
    }

    fn ordered_bodies<'a>(&self, bodies: &'a [Body]) -> Vec<&'a Body> {
        let mut ordered = bodies.iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| self.compare(left, right));
        ordered
    }

    fn compare(&self, left: &Body, right: &Body) -> Ordering {
        let ordering = match self.sort_mode {
            SortMode::Distance => compare_optional(left.distance_ly, right.distance_ly),
            SortMode::Temperature => {
                compare_optional(left.surface_temperature, right.surface_temperature)
            }
            SortMode::Pressure => compare_optional(left.surface_pressure, right.surface_pressure),
            SortMode::Gravity => compare_optional(left.gravity, right.gravity),
            SortMode::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        };
        let ordering = if self.descending {
            match self.sort_mode {
                SortMode::Distance => compare_optional_desc(left.distance_ly, right.distance_ly),
                SortMode::Temperature => {
                    compare_optional_desc(left.surface_temperature, right.surface_temperature)
                }
                SortMode::Pressure => {
                    compare_optional_desc(left.surface_pressure, right.surface_pressure)
                }
                SortMode::Gravity => compare_optional_desc(left.gravity, right.gravity),
                SortMode::Name => left
                    .name
                    .to_lowercase()
                    .cmp(&right.name.to_lowercase())
                    .reverse(),
            }
        } else {
            ordering
        };
        ordering.then_with(|| left.name.cmp(&right.name))
    }

    fn selected_index(&self, ordered: &[&Body]) -> usize {
        self.selected_identity
            .as_deref()
            .and_then(|identity| {
                ordered
                    .iter()
                    .position(|body| body_identity(body) == identity)
            })
            .unwrap_or_default()
            .min(ordered.len().saturating_sub(1))
    }

    fn select(&mut self, body: &Body) {
        self.selected_identity = Some(body_identity(body));
    }

    fn selected_body<'a>(&self, ordered: &[&'a Body]) -> Option<&'a Body> {
        ordered.get(self.selected_index(ordered)).copied()
    }

    pub fn detail_scroll(&self) -> u16 {
        self.detail_scroll
    }

    fn sort_label(&self) -> String {
        let arrow = if self.descending { "▼" } else { "▲" };
        format!("{} {arrow}", self.sort_mode.label())
    }

    fn header_label(&self, mode: SortMode, label: &str) -> String {
        if self.sort_mode == mode {
            format!("{label} {}", if self.descending { "▼" } else { "▲" })
        } else {
            label.to_owned()
        }
    }
}

fn compare_optional(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_desc(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn body_identity(body: &Body) -> String {
    body.id64.clone().unwrap_or_else(|| {
        format!(
            "{}:{}",
            body.system_id64.as_deref().unwrap_or(""),
            body.name
        )
    })
}

fn display_number(value: Option<f64>, precision: usize) -> String {
    value.map_or_else(|| "—".to_owned(), |value| format!("{value:.precision$}"))
}

fn revenue(view: Option<&ExobiologyView>) -> String {
    view.map_or_else(
        || "—".to_owned(),
        |view| format!("{} CR", format_credits(view.maximum_base_value_cr)),
    )
}

pub fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
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

    if app.results.is_empty() {
        render_empty(frame, app);
        return;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(format!(
            "{}: {}  ·  СИСТЕМА: {}  ·  СОРТИРОВКА: {}",
            ui::RESULTS_SUMMARY_LABEL,
            app.results.len(),
            app.config.system,
            app.results_screen.sort_label()
        ))
        .alignment(Alignment::Center)
        .style(theme::Theme::default().accent)
        .block(Block::default().borders(Borders::ALL)),
        root[0],
    );

    let wide = area.width >= 120;
    let panes = if wide {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(root[1])
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(root[1])
    };

    render_planet_table(frame, panes[0], app);
    render_details(frame, panes[1], app);

    let hint = app
        .results_screen
        .status_message
        .as_deref()
        .map_or_else(|| ui::RESULTS_HINTS, |status| status);
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(theme::Theme::default().muted)
            .block(Block::default().borders(Borders::TOP)),
        root[2],
    );
}

fn render_empty(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = theme::centered(
        frame.area(),
        frame.area().width.saturating_sub(4),
        frame.area().height.saturating_sub(4),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{}\n\n{}: {}\n\n{}\n\n{}",
            ui::RESULTS_EMPTY_MESSAGE,
            ui::RESULTS_EMPTY_SYSTEM_LABEL,
            app.config.system,
            ui::RESULTS_EMPTY_HINT,
            ui::RESULTS_EMPTY_ACTIONS
        ))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(ui::RESULTS_EMPTY_TITLE)
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_planet_table(frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let ordered = app.results_screen.ordered_bodies(&app.results);
    let selected = app.results_screen.selected_index(&ordered);
    let rows = ordered
        .iter()
        .map(|body| {
            let view = exobiology_view(body).ok().flatten();
            Row::new(vec![
                Cell::from(body.name.as_str()),
                Cell::from(body.system_name.as_deref().unwrap_or("—")),
                Cell::from(display_number(body.distance_ly, 1)),
                Cell::from(display_number(body.surface_temperature, 0)),
                Cell::from(display_number(body.surface_pressure, 3)),
                Cell::from(display_number(body.gravity, 2)),
                Cell::from(revenue(view.as_ref())),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = TableState::default();
    state.select(Some(selected));
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(23),
            Constraint::Percentage(20),
            Constraint::Percentage(9),
            Constraint::Percentage(8),
            Constraint::Percentage(10),
            Constraint::Percentage(8),
            Constraint::Percentage(22),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from(
                app.results_screen
                    .header_label(SortMode::Name, ui::RESULTS_PLANET_HEADER),
            ),
            Cell::from(ui::RESULTS_SYSTEM_HEADER),
            Cell::from(
                app.results_screen
                    .header_label(SortMode::Distance, ui::RESULTS_DISTANCE_HEADER),
            ),
            Cell::from(
                app.results_screen
                    .header_label(SortMode::Temperature, ui::RESULTS_TEMPERATURE_HEADER),
            ),
            Cell::from(
                app.results_screen
                    .header_label(SortMode::Pressure, ui::RESULTS_PRESSURE_HEADER),
            ),
            Cell::from(
                app.results_screen
                    .header_label(SortMode::Gravity, ui::RESULTS_GRAVITY_HEADER),
            ),
            Cell::from(ui::RESULTS_REVENUE_HEADER),
        ])
        .style(theme::Theme::default().accent)
        .bottom_margin(0),
    )
    .row_highlight_style(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .bg(ratatui::style::Color::DarkGray),
    )
    .highlight_symbol("▸ ")
    .block(
        Block::default()
            .title(ui::RESULTS_TABLE_TITLE)
            .borders(Borders::ALL),
    );

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_details(frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let ordered = app.results_screen.ordered_bodies(&app.results);
    let Some(body) = app.results_screen.selected_body(&ordered) else {
        return;
    };
    let details = vec![
        Line::from(format!("{}: {}", ui::RESULTS_DETAIL_PLANET, body.name)),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_SYSTEM,
            body.system_name
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_SURFACE,
            body.subtype
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_ATMOSPHERE,
            body.atmosphere
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
        Line::from(format!(
            "{}: {} K",
            ui::RESULTS_DETAIL_TEMPERATURE,
            display_number(body.surface_temperature, 2)
        )),
        Line::from(format!(
            "{}: {} atm",
            ui::RESULTS_DETAIL_PRESSURE,
            display_number(body.surface_pressure, 4)
        )),
        Line::from(format!(
            "{}: {} g",
            ui::RESULTS_DETAIL_GRAVITY,
            display_number(body.gravity, 3)
        )),
        Line::from(format!(
            "{}: {} св. л.",
            ui::RESULTS_DETAIL_DISTANCE,
            display_number(body.distance_ly, 2)
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_LANDING,
            body.is_landable
                .map_or(ui::RESULTS_MISSING_VALUE, |value| if value {
                    ui::RESULTS_YES
                } else {
                    ui::RESULTS_NO
                })
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_MAPPED,
            body.is_mapped
                .map_or(ui::RESULTS_MISSING_VALUE, |value| if value {
                    ui::RESULTS_YES
                } else {
                    ui::RESULTS_NO
                })
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_DATA_UPDATED,
            body.last_updated_at
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_SIGNALS_UPDATED,
            body.signals_updated_at
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_BIO_COUNT,
            body.biological_signal_count.map_or_else(
                || ui::RESULTS_MISSING_VALUE.to_owned(),
                |value| value.to_string()
            )
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_GEO_COUNT,
            body.geological_signal_count.map_or_else(
                || ui::RESULTS_MISSING_VALUE.to_owned(),
                |value| value.to_string()
            )
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_BIO_GROUPS,
            body.biological_genera
                .as_ref()
                .filter(|values| !values.is_empty())
                .map_or_else(
                    || ui::RESULTS_MISSING_VALUE.to_owned(),
                    |values| values
                        .iter()
                        .map(|value| crate::presentation::text::russian_group_name(value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
        )),
        Line::from(format!(
            "{}: {}",
            ui::RESULTS_DETAIL_VOLCANISM,
            body.volcanism_type
                .as_deref()
                .unwrap_or(ui::RESULTS_MISSING_VALUE)
        )),
    ];

    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);
    frame.render_widget(
        Paragraph::new(details)
            .scroll((app.results_screen.detail_scroll(), 0))
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(ui::RESULTS_DETAILS_TITLE)
                    .borders(Borders::ALL),
            ),
        detail_layout[0],
    );

    let view = exobiology_view(body).ok().flatten();
    render_exobiology(frame, detail_layout[1], view.as_ref());
}

fn render_exobiology(
    frame: &mut ratatui::Frame<'_>,
    area: ratatui::layout::Rect,
    view: Option<&ExobiologyView>,
) {
    let Some(view) = view else {
        frame.render_widget(
            Paragraph::new(ui::RESULTS_EXOBIOLOGY_EMPTY)
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(ui::RESULTS_EXOBIOLOGY_TITLE)
                        .borders(Borders::ALL),
                ),
            area,
        );
        return;
    };

    if view.rows.is_empty() {
        let mut lines = vec![Line::from(view.headline.as_str())];
        lines.extend(view.summary.iter().map(|line| Line::from(line.as_str())));
        lines.extend(view.notes.iter().map(|line| Line::from(line.as_str())));
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(ui::RESULTS_EXOBIOLOGY_TITLE)
                    .borders(Borders::ALL),
            ),
            area,
        );
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(2),
            Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(view.headline.as_str())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(ui::RESULTS_EXOBIOLOGY_TITLE)
                    .borders(Borders::ALL),
            ),
        sections[0],
    );

    let rows = view
        .rows
        .iter()
        .map(|row| {
            Row::new(vec![
                Cell::from(row.genus.as_str()),
                Cell::from(row.species.as_str()),
                Cell::from(format!("{} CR", format_credits(row.base_value_cr))),
                Cell::from(row.status.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(22),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(42),
        ],
    )
    .header(Row::new([
        ui::RESULTS_EXOBIOLOGY_GROUP_HEADER,
        ui::RESULTS_EXOBIOLOGY_SPECIES_HEADER,
        ui::RESULTS_EXOBIOLOGY_VALUE_HEADER,
        ui::RESULTS_EXOBIOLOGY_STATUS_HEADER,
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(table, sections[1]);

    let footer = view
        .summary
        .iter()
        .chain(view.notes.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join(" · ");
    frame.render_widget(
        Paragraph::new(footer)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL)),
        sections[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::criteria::{Filter, NumericRange, SearchCriteria};

    fn body(name: &str, id: &str, distance: Option<f64>) -> Body {
        Body {
            id64: Some(id.to_owned()),
            name: name.to_owned(),
            system_id64: Some("42".to_owned()),
            system_name: Some("Sol".to_owned()),
            subtype: Some("Rocky body".to_owned()),
            distance_ly: distance,
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(200.0),
            surface_pressure: Some(0.5),
            gravity: Some(0.2),
            atmosphere: Some("Water".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    fn app_with_bodies(bodies: Vec<Body>) -> App {
        let criteria = SearchCriteria::new(
            crate::domain::criteria::Filter::Any,
            Filter::Any,
            NumericRange {
                min: 0.0,
                max: 5000.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.0,
                max: 10.0,
            },
            5,
        )
        .unwrap();
        let mut app =
            App::new(crate::application::config::SearchConfig::new("Sol", criteria).unwrap());
        app.results = bodies;
        app.results_screen.reset(&app.results);
        app.screen = crate::presentation::app::Screen::Results;
        app
    }

    #[test]
    fn selection_is_preserved_when_sort_order_changes() {
        let mut near = body("Near", "1", Some(1.0));
        let mut far = body("Far", "2", Some(10.0));
        near.surface_temperature = Some(200.0);
        far.surface_temperature = Some(100.0);
        let bodies = vec![near, far];
        let mut screen = ResultsScreen::default();
        screen.reset(&bodies);

        screen.handle_key(
            KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE),
            &bodies,
        );
        screen.handle_key(
            KeyEvent::new(KeyCode::Char('s'), crossterm::event::KeyModifiers::NONE),
            &bodies,
        );

        let reordered = screen.ordered_bodies(&bodies);
        assert_eq!(
            screen.selected_body(&reordered).unwrap().id64.as_deref(),
            Some("2")
        );
        assert_eq!(screen.selected_index(&reordered), 0);
    }

    #[test]
    fn sort_cycles_through_all_five_columns() {
        let mut screen = ResultsScreen::default();
        let bodies = vec![body("Near", "1", Some(1.0))];

        for expected in [
            SortMode::Temperature,
            SortMode::Pressure,
            SortMode::Gravity,
            SortMode::Name,
            SortMode::Distance,
        ] {
            screen.handle_key(
                KeyEvent::new(KeyCode::Char('s'), crossterm::event::KeyModifiers::NONE),
                &bodies,
            );
            assert_eq!(screen.sort_mode, expected);
        }
    }

    #[test]
    fn copy_action_uses_the_selected_planets_system() {
        let bodies = vec![body("Near", "1", Some(1.0)), body("Far", "2", Some(10.0))];
        let mut screen = ResultsScreen::default();
        screen.reset(&bodies);
        screen.handle_key(
            KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE),
            &bodies,
        );

        assert_eq!(
            screen.handle_key(
                KeyEvent::new(KeyCode::Char('c'), crossterm::event::KeyModifiers::NONE),
                &bodies
            ),
            Some(ResultsAction::CopySystem("Sol".to_owned()))
        );
    }

    #[test]
    fn empty_results_explain_what_to_change_and_how_to_continue() {
        let app = app_with_bodies(Vec::new());
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("Подходящие планеты не найдены"));
        assert!(output.contains("температуры"));
        assert!(output.contains("биосигналам"));
        assert!(output.contains("e / у — изменить параметры"));
        assert!(output.contains("n / т — новый поиск"));
    }

    #[test]
    fn missing_numeric_values_remain_last_in_both_sort_directions() {
        let bodies = vec![body("Missing", "1", None), body("Known", "2", Some(2.0))];
        let mut screen = ResultsScreen::default();

        assert_eq!(screen.ordered_bodies(&bodies)[0].name, "Known");
        screen.descending = true;
        assert_eq!(screen.ordered_bodies(&bodies)[1].name, "Missing");
    }

    #[test]
    fn renderer_shows_search_context_and_no_missing_data_placeholders() {
        let app = app_with_bodies(vec![body("Planet A", "1", Some(12.5))]);
        let backend = ratatui::backend::TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("Planet A"));
        assert!(output.contains("Sol"));
        assert!(output.contains("12.5"));
        assert!(output.contains("Атмосфера"));
        assert!(output.contains("Температура"));
        assert!(output.contains("Гравитация"));
        assert!(output.contains("Биосигналы Spansh"));
        assert!(output.contains("Вулканизм"));
        assert!(!output.contains("нет данных"));
    }

    #[test]
    fn renderer_supports_the_minimum_terminal_size() {
        let app = app_with_bodies(vec![body("Planet A", "1", Some(12.5))]);
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("Планета"));
        assert!(output.contains("Planet A"));
        assert!(!output.contains("нет данных"));
    }
}
