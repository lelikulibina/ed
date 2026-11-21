use std::{
    collections::HashSet,
    io::{self, Stdout},
};

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
    widgets::{Block, Borders, ListState, Paragraph, Wrap},
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

    let mut sorted_bodies = bodies.to_vec();
    let mut sort_mode = 0usize;
    sort_bodies(&mut sorted_bodies, sort_mode);
    let mut selected = 0usize;
    let mut detail_scroll = 0u16;
    let mut details_open = false;
    let mut marked = HashSet::<String>::new();

    loop {
        session.terminal.draw(|frame| {
            draw_results(
                frame,
                &sorted_bodies,
                selected,
                detail_scroll,
                sort_mode,
                &marked,
                details_open,
            )
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
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(SearchUiAction::Exit);
            }
            KeyCode::Esc => {
                if details_open {
                    details_open = false;
                    detail_scroll = 0;
                } else {
                    return Ok(SearchUiAction::BackToSetup);
                }
            }
            KeyCode::Enter => {
                details_open = !details_open;
                detail_scroll = 0;
            }
            KeyCode::Up if details_open => detail_scroll = detail_scroll.saturating_sub(1),
            KeyCode::Down if details_open => detail_scroll = detail_scroll.saturating_add(1),
            KeyCode::PageUp if details_open => detail_scroll = detail_scroll.saturating_sub(8),
            KeyCode::PageDown if details_open => detail_scroll = detail_scroll.saturating_add(8),
            KeyCode::Up if !sorted_bodies.is_empty() => selected = selected.saturating_sub(1),
            KeyCode::Down if !sorted_bodies.is_empty() => {
                selected = (selected + 1).min(sorted_bodies.len() - 1)
            }
            KeyCode::Left if !details_open => {
                sort_mode = (sort_mode + 4) % 5;
                sort_bodies(&mut sorted_bodies, sort_mode);
                selected = 0;
            }
            KeyCode::Right if !details_open => {
                sort_mode = (sort_mode + 1) % 5;
                sort_bodies(&mut sorted_bodies, sort_mode);
                selected = 0;
            }
            KeyCode::Home if !details_open => selected = 0,
            KeyCode::End if !details_open && !sorted_bodies.is_empty() => {
                selected = sorted_bodies.len() - 1
            }
            KeyCode::Char(' ') => {
                if let Some(body) = sorted_bodies.get(selected) {
                    if !marked.remove(&body.name) {
                        marked.insert(body.name.clone());
                    }
                }
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

    bodies.sort_by(|left, right| match mode {
        0 => compare_optional(left.distance_ly, right.distance_ly),
        1 => compare_optional(left.surface_temperature, right.surface_temperature),
        2 => compare_optional(left.gravity, right.gravity),
        3 => compare_optional(left.surface_pressure, right.surface_pressure),
        _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    });
}

fn draw_results(
    frame: &mut ratatui::Frame<'_>,
    bodies: &[crate::domain::body::Body],
    selected: usize,
    detail_scroll: u16,
    sort_mode: usize,
    marked: &HashSet<String>,
    details_open: bool,
) {
    use ratatui::widgets::{List, ListItem};

    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(format!(
            "ED: PLANET SEARCH  /  РЕЗУЛЬТАТЫ: {}  /  ОТМЕЧЕНО: {}",
            bodies.len(),
            marked.len()
        ))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
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

    if !details_open {
        let items = bodies
            .iter()
            .map(|body| {
                let marked_label = if marked.contains(&body.name) {
                    "★ "
                } else {
                    ""
                };
                let mapped = if body.is_mapped == Some(true) {
                    "  [картографирована]"
                } else {
                    ""
                };
                let label = format!("{marked_label}{}{mapped}", body.name);
                let style = if body.is_mapped == Some(true) {
                    Style::default().fg(Color::LightGreen)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(label).style(style)
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        if !bodies.is_empty() {
            state.select(Some(selected.min(bodies.len() - 1)));
        }
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▸ ")
                .block(
                    Block::default()
                        .title(" НАЙДЕННЫЕ ПЛАНЕТЫ ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                ),
            panes[0],
            &mut state,
        );
    }

    let detail = bodies.get(selected).map_or_else(
        || "Подходящие планеты не найдены.".to_owned(),
        |body| {
            let mut lines = vec![
                format!("Планета: {}", body.name),
                format!(
                    "Система: {}",
                    body.system_name.as_deref().unwrap_or("нет данных")
                ),
                format!("Тип: {}", body.subtype.as_deref().unwrap_or("нет данных")),
                format!(
                    "Атмосфера: {}",
                    body.atmosphere.as_deref().unwrap_or("нет данных")
                ),
                format!(
                    "Температура: {}",
                    body.surface_temperature
                        .map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.2} K"))
                ),
                format!(
                    "Давление: {}",
                    body.surface_pressure
                        .map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.4} atm"))
                ),
                format!(
                    "Гравитация: {}",
                    body.gravity
                        .map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.3} g"))
                ),
                format!(
                    "Расстояние: {}",
                    body.distance_ly
                        .map_or_else(|| "нет данных".to_owned(), |v| format!("{v:.2} св. л."))
                ),
                format!(
                    "Биологические сигналы Spansh: {}",
                    body.biological_signal_count
                        .map_or_else(|| "нет данных".to_owned(), |v| v.to_string())
                ),
                format!(
                    "Геологические сигналы Spansh: {}",
                    body.geological_signal_count
                        .map_or_else(|| "нет данных".to_owned(), |v| v.to_string())
                ),
                format!(
                    "Биологические группы: {}",
                    body.biological_genera
                        .as_ref()
                        .filter(|v| !v.is_empty())
                        .map_or_else(
                            || "не указаны".to_owned(),
                            |v| v
                                .iter()
                                .map(|g| crate::presentation::output::russian_group_name(g))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                ),
                format!(
                    "Вулканизм: {}",
                    body.volcanism_type.as_deref().unwrap_or("нет данных")
                ),
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
        Paragraph::new(detail)
            .scroll((detail_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" ПОДРОБНОСТИ ПЛАНЕТЫ ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            ),
        if details_open { root[1] } else { panes[1] },
    );
    let sort_label = [
        "расстояние",
        "температура",
        "гравитация",
        "давление",
        "название",
    ]
    .get(sort_mode)
    .copied()
    .unwrap_or("расстояние");
    let hint = if details_open {
        "↑/↓ прокрутка   Enter/Esc — к списку   Space — отметить планету   Ctrl+Q — выход"
            .to_owned()
    } else {
        format!(
            "↑/↓ планета   ←/→ сортировка: {sort_label}   Enter — подробности   Space — отметить   Esc — настройки   Ctrl+Q — выход"
        )
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
        root[2],
    );
}
