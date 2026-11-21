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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use crate::application::config::SearchConfig;

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

pub fn run_tui(initial: Option<SearchConfig>) -> Result<Option<SearchConfig>> {
    super::setup::run_setup(initial)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchUiAction {
    BackToSetup,
    Exit,
}

/// Выполняет асинхронный поиск, не блокируя полноэкранный интерфейс.
pub async fn run_search_ui<F>(future: F) -> Result<SearchUiAction>
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
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if key.code == KeyCode::Esc {
                task.abort();
                return Ok(SearchUiAction::BackToSetup);
            } else if key.code == KeyCode::Char('q')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                task.abort();
                return Ok(SearchUiAction::Exit);
            }
        }
    };

    match outcome {
        Ok(bodies) => show_results_in_session(&mut session, &bodies),
        Err(error) => show_error_in_session(&mut session, &error.to_string()),
    }
}

fn show_error_in_session(session: &mut TerminalSession, message: &str) -> Result<SearchUiAction> {
    loop {
        session.terminal.draw(|frame| {
            let panel = centered(75, 40, frame.area());
            frame.render_widget(
                Paragraph::new(format!(
                    "Поиск не выполнен\n\n{message}\n\nEnter / Esc — вернуться к настройкам   Ctrl+Q — выход"
                ))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(Block::default().title(" ОШИБКА ПОИСКА ").borders(Borders::ALL).border_style(Style::default().fg(Color::Red))),
                panel,
            );
        })?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                return Ok(SearchUiAction::BackToSetup);
            } else if key.code == KeyCode::Char('q')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                return Ok(SearchUiAction::Exit);
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
) -> Result<SearchUiAction> {
    use std::time::Duration;

    use super::{
        layout::{LayoutMode, layout_mode},
        results::{ResultsState, SortColumn, body_identity, sort_bodies},
    };

    let mut sorted_bodies = bodies.to_vec();
    let mut state = ResultsState::new(&sorted_bodies);
    sort_bodies(&mut sorted_bodies, state.sort_column, state.sort_direction);

    loop {
        session
            .terminal
            .draw(|frame| draw_results(frame, &sorted_bodies, &state))?;
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
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(SearchUiAction::Exit);
            }
            KeyCode::Esc => {
                if state.details_open {
                    state.details_open = false;
                    state.detail_scroll = 0;
                } else {
                    return Ok(SearchUiAction::BackToSetup);
                }
            }
            KeyCode::Enter => {
                state.details_open = !state.details_open;
                state.detail_scroll = 0;
            }
            KeyCode::Up if state.details_open => {
                state.detail_scroll = state.detail_scroll.saturating_sub(1);
            }
            KeyCode::Down if state.details_open => {
                state.detail_scroll = state.detail_scroll.saturating_add(1);
            }
            KeyCode::PageUp if state.details_open => {
                state.detail_scroll = state.detail_scroll.saturating_sub(8);
            }
            KeyCode::PageDown if state.details_open => {
                state.detail_scroll = state.detail_scroll.saturating_add(8);
            }
            KeyCode::Up => state.select_previous(),
            KeyCode::Down => state.select_next(sorted_bodies.len()),
            KeyCode::Home if !state.details_open => state.select_first(),
            KeyCode::End if !state.details_open => state.select_last(sorted_bodies.len()),
            KeyCode::Char('s') if !state.details_open => {
                let selected_identity = sorted_bodies
                    .get(state.selected)
                    .map(body_identity);
                state.sort_column = state.sort_column.next();
                sort_bodies(&mut sorted_bodies, state.sort_column, state.sort_direction);
                restore_selection(&mut state, &sorted_bodies, selected_identity.as_deref());
            }
            KeyCode::Char('r') if !state.details_open => {
                let selected_identity = sorted_bodies
                    .get(state.selected)
                    .map(body_identity);
                state.sort_direction.toggle();
                sort_bodies(&mut sorted_bodies, state.sort_column, state.sort_direction);
                restore_selection(&mut state, &sorted_bodies, selected_identity.as_deref());
            }
            KeyCode::Char(' ') if !state.details_open => {
                if let Some(body) = sorted_bodies.get(state.selected) {
                    state.toggle_mark(body);
                }
            }
            _ => {}
        }
        let _ = layout_mode(session.terminal.size()?);
    }
}

fn restore_selection(
    state: &mut super::results::ResultsState,
    bodies: &[crate::domain::body::Body],
    identity: Option<&str>,
) {
    state.selected = identity
        .and_then(|identity| {
            bodies
                .iter()
                .position(|body| body_identity(body) == identity)
        })
        .unwrap_or(0);
}

fn draw_results(
    frame: &mut ratatui::Frame<'_>,
    bodies: &[crate::domain::body::Body],
    state: &super::results::ResultsState,
) {
    use super::{
        layout::{LayoutMode, layout_mode},
        results::{SortColumn, cell_value, visible_columns},
    };

    let area = frame.area();
    let mode = layout_mode(area);
    if mode == LayoutMode::TooSmall {
        frame.render_widget(
            Paragraph::new("Терминал слишком мал. Увеличьте окно минимум до 64×18.")
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::Yellow)),
            area,
        );
        return;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(format!(
            "ED  /  PLANET SEARCH     РЕЗУЛЬТАТЫ {}     ОТМЕЧЕНО {}",
            bodies.len(),
            state.marked.len()
        ))
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        root[0],
    );

    if state.details_open {
        let detail = bodies
            .get(state.selected)
            .map_or_else(|| "Подходящие планеты не найдены.".to_owned(), body_detail_text);
        frame.render_widget(
            Paragraph::new(detail)
                .scroll((state.detail_scroll, 0))
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .title(" ПОДРОБНОСТИ ПЛАНЕТЫ ")
                        .borders(Borders::LEFT)
                        .border_style(Style::default().fg(Color::Magenta)),
                ),
            root[1],
        );
    } else {
        let columns = visible_columns(mode);
        let (table_area, inspector_area) = if mode == LayoutMode::Wide {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
                .split(root[1]);
            (panes[0], Some(panes[1]))
        } else {
            (root[1], None)
        };

        if bodies.is_empty() {
            frame.render_widget(
                Paragraph::new("Совпадений не найдено. Измените критерии и повторите поиск.")
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(Color::Gray)),
                table_area,
            );
        } else {
            let header = Row::new(columns.iter().map(|column| {
                let marker = if *column == state.sort_column {
                    state.sort_direction.indicator()
                } else {
                    ""
                };
                Cell::from(format!("{}{marker}", column.label()))
            }))
            .style(
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            );

            let rows = bodies.iter().map(|body| {
                let mut cells = columns
                    .iter()
                    .map(|column| Cell::from(cell_value(body, *column)))
                    .collect::<Vec<_>>();
                if let Some(first) = cells.first_mut()
                    && state.is_marked(body)
                {
                    *first = Cell::from(format!("★ {}", cell_value(body, SortColumn::Name)));
                }
                Row::new(cells).style(if body.is_mapped == Some(true) {
                    Style::default().fg(Color::LightGreen)
                } else {
                    Style::default().fg(Color::White)
                })
            });

            let widths = columns
                .iter()
                .map(|column| match column {
                    SortColumn::Name => Constraint::Min(18),
                    SortColumn::Surface | SortColumn::Atmosphere => Constraint::Length(15),
                    SortColumn::Temperature | SortColumn::Pressure => Constraint::Length(11),
                    SortColumn::Gravity => Constraint::Length(9),
                    SortColumn::Distance => Constraint::Length(12),
                })
                .collect::<Vec<_>>();

            let table = Table::new(rows, widths)
                .header(header)
                .column_spacing(1)
                .row_highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▸ ")
                .block(
                    Block::default()
                        .title(" НАЙДЕННЫЕ МИРЫ ")
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
            let mut table_state = TableState::default();
            table_state.select(Some(state.selected.min(bodies.len() - 1)));
            frame.render_stateful_widget(table, table_area, &mut table_state);
        }

        if let Some(inspector_area) = inspector_area {
            let content = bodies
                .get(state.selected)
                .map_or_else(|| "Нет выбранной планеты.".to_owned(), body_summary_text);
            frame.render_widget(
                Paragraph::new(content)
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .title(" ИНСПЕКТОР ")
                            .borders(Borders::LEFT)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    ),
                inspector_area,
            );
        }
    }

    let hint = if state.details_open {
        "↑/↓ прокрутка   PgUp/PgDn страница   Esc — к таблице   Ctrl+Q — выход"
    } else {
        "↑/↓ выбор   s сортировка   r направление   Space отметить   Enter подробности   Esc настройки   Ctrl+Q выход"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Gray)),
        root[2],
    );
}

fn body_summary_text(body: &crate::domain::body::Body) -> String {
    use super::results::format_optional;

    format!(
        "{}\n{}\n\nПоверхность  {}\nАтмосфера    {}\nТемпература  {}\nРасстояние   {}\n\nКартографирована  {}\nБиосигналы        {}",
        body.name,
        body.system_name.as_deref().unwrap_or("Система неизвестна"),
        body.subtype.as_deref().unwrap_or("—"),
        body.atmosphere.as_deref().unwrap_or("—"),
        format_optional(body.surface_temperature, 1, " K"),
        format_optional(body.distance_ly, 1, " св. л."),
        match body.is_mapped {
            Some(true) => "да",
            Some(false) => "нет",
            None => "нет данных",
        },
        body.biological_signal_count.map_or_else(|| "нет данных".to_owned(), |v| v.to_string())
    )
}

fn body_detail_text(body: &crate::domain::body::Body) -> String {
    use super::results::format_optional;

    let mut lines = vec![
        format!("{}  /  {}", body.name, body.system_name.as_deref().unwrap_or("система неизвестна")),
        format!("ID тела: {}", body.id64.as_deref().unwrap_or("нет данных")),
        format!("Тип поверхности: {}", body.subtype.as_deref().unwrap_or("нет данных")),
        format!("Атмосфера: {}", body.atmosphere.as_deref().unwrap_or("нет данных")),
        format!("Температура: {}", format_optional(body.surface_temperature, 2, " K")),
        format!("Давление: {}", format_optional(body.surface_pressure, 4, " atm")),
        format!("Гравитация: {}", format_optional(body.gravity, 3, " g")),
        format!("Расстояние: {}", format_optional(body.distance_ly, 2, " св. л.")),
        format!("Можно приземлиться: {}", optional_bool(body.is_landable)),
        format!("Картографирована: {}", optional_bool(body.is_mapped)),
        format!("Обновление записи: {}", body.last_updated_at.as_deref().unwrap_or("нет данных")),
        format!("Биологические сигналы Spansh: {}", body.biological_signal_count.map_or_else(|| "нет данных".to_owned(), |v| v.to_string())),
        format!("Геологические сигналы Spansh: {}", body.geological_signal_count.map_or_else(|| "нет данных".to_owned(), |v| v.to_string())),
        format!(
            "Биологические группы: {}",
            body.biological_genera
                .as_ref()
                .filter(|genera| !genera.is_empty())
                .map_or_else(|| "не указаны".to_owned(), |genera| genera
                    .iter()
                    .map(|genus| crate::presentation::output::russian_group_name(genus))
                    .collect::<Vec<_>>()
                    .join(", "))
        ),
        format!("Вулканизм: {}", body.volcanism_type.as_deref().unwrap_or("нет данных")),
        String::new(),
    ];
    match crate::presentation::output::exobiology_summary(body) {
        Ok(Some(summary)) => lines.extend(summary.lines().map(str::to_owned)),
        Ok(None) => lines.push("Подходящие виды в справочнике не найдены.".to_owned()),
        Err(error) => lines.push(format!("Ошибка расчёта экзобиологии: {error}")),
    }
    lines.join("\n")
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "да",
        Some(false) => "нет",
        None => "нет данных",
    }
}
