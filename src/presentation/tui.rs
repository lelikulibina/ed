use std::collections::HashSet;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, ListState, Paragraph, Wrap},
};
use tokio::sync::mpsc;

use crate::{
    application::{
        config::SearchConfig,
        search::{SearchError, SearchEvent},
    },
    domain::body::Body,
};

use super::{
    app::{self, App, AppEvent, Command, Screen},
    events::poll_and_send,
    terminal::TerminalGuard,
};

pub async fn run<F, Fut>(initial: Option<SearchConfig>, search: F) -> Result<()>
where
    F: Fn(SearchConfig, mpsc::UnboundedSender<SearchEvent>) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Vec<Body>, SearchError>> + Send + 'static,
{
    let mut session = TerminalGuard::enter()?;
    let (sender, mut receiver) = mpsc::channel(32);
    let mut app = initial
        .clone()
        .map(App::new)
        .unwrap_or_else(App::new_default);

    if let Some(config) = initial {
        app.screen = Screen::Running;
        start_search_task(&mut app, config, sender.clone(), &search).await;
    }

    loop {
        session
            .terminal_mut()
            .draw(|frame| app::render(frame, &app))?;
        poll_and_send(&sender, std::time::Duration::from_millis(100), false)?;

        if let Ok(event) = receiver.try_recv() {
            let terminal_event = matches!(
                &event,
                AppEvent::Search(SearchEvent::Finished(_) | SearchEvent::Failed(_))
            );
            let command = app::update(&mut app, event);

            if terminal_event && let Some(task) = app.search_task.take() {
                let _ = task.await;
            }

            match command {
                Some(Command::StartSearch(config)) => {
                    start_search_task(&mut app, config, sender.clone(), &search).await;
                }
                Some(Command::CancelSearch) => {
                    cancel_search_task(&mut app).await;
                    while receiver.try_recv().is_ok() {}
                }
                Some(Command::Quit) => {
                    cancel_search_task(&mut app).await;
                    break;
                }
                Some(Command::CopySystem(_)) | None => {}
            }
        }

        if app.screen == Screen::Results {
            match show_results_in_session(&mut session, &app.results)? {
                SearchUiAction::BackToSetup => app.screen = Screen::Search,
                SearchUiAction::Exit => break,
            }
        }
    }

    cancel_search_task(&mut app).await;
    Ok(())
}

async fn start_search_task<F, Fut>(
    app: &mut App,
    config: SearchConfig,
    sender: mpsc::Sender<AppEvent>,
    search: &F,
) where
    F: Fn(SearchConfig, mpsc::UnboundedSender<SearchEvent>) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Vec<Body>, SearchError>> + Send + 'static,
{
    cancel_search_task(app).await;
    app.screen = Screen::Running;
    app.config = config.clone();
    app.progress_message = Some("Подготовка поиска…".to_owned());
    app.resolved_system = None;
    app.current_page = 0;
    app.found_count = 0;

    let search = search.clone();
    app.search_task = Some(tokio::spawn(async move {
        let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();
        let mut search_future = Box::pin(search(config, progress_sender.clone()));
        let result = loop {
            tokio::select! {
                result = &mut search_future => break result,
                event = progress_receiver.recv() => {
                    if let Some(event) = event
                        && sender.send(AppEvent::Search(event)).await.is_err()
                    {
                        return;
                    }
                }
            }
        };

        drop(search_future);
        drop(progress_sender);
        while let Some(event) = progress_receiver.recv().await {
            if sender.send(AppEvent::Search(event)).await.is_err() {
                return;
            }
        }

        let event = match result {
            Ok(results) => SearchEvent::Finished(results),
            Err(error) => SearchEvent::Failed(error),
        };
        let _ = sender.send(AppEvent::Search(event)).await;
    }));
}

async fn cancel_search_task(app: &mut App) {
    if let Some(task) = app.search_task.take() {
        task.abort();
        let _ = task.await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchUiAction {
    BackToSetup,
    Exit,
}

fn show_results_in_session(
    session: &mut TerminalGuard,
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
        session.terminal_mut().draw(|frame| {
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
                if let Some(body) = sorted_bodies.get(selected)
                    && !marked.remove(&body.name)
                {
                    marked.insert(body.name.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn search_task_forwards_ordered_progress_before_finished_results() {
        let mut app = App::new_default();
        let (sender, mut receiver) = mpsc::channel(8);
        let search = |_: SearchConfig, events: mpsc::UnboundedSender<SearchEvent>| async move {
            events.send(SearchEvent::ResolvingSystem).unwrap();
            events
                .send(SearchEvent::SystemResolved {
                    name: "Sol".to_owned(),
                })
                .unwrap();
            events
                .send(SearchEvent::PageLoaded { page: 1, found: 0 })
                .unwrap();
            Ok(Vec::new())
        };

        let config = app.config.clone();
        start_search_task(&mut app, config, sender, &search).await;

        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap(),
            Some(AppEvent::Search(SearchEvent::ResolvingSystem))
        ));
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap(),
            Some(AppEvent::Search(SearchEvent::SystemResolved { name })) if name == "Sol"
        ));
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap(),
            Some(AppEvent::Search(SearchEvent::PageLoaded { page: 1, found: 0 }))
        ));
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap(),
            Some(AppEvent::Search(SearchEvent::Finished(results))) if results.is_empty()
        ));

        if let Some(task) = app.search_task.take() {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn replacing_a_search_aborts_and_joins_the_previous_task() {
        let mut app = App::new_default();
        let previous_cancelled = Arc::new(AtomicBool::new(false));
        let previous_flag = Arc::clone(&previous_cancelled);
        app.search_task = Some(tokio::spawn(async move {
            let _guard = DropFlag(previous_flag);
            std::future::pending::<()>().await;
        }));
        tokio::task::yield_now().await;

        let (sender, _receiver) = mpsc::channel(8);
        let search = |_: SearchConfig, _: mpsc::UnboundedSender<SearchEvent>| async {
            std::future::pending::<Result<Vec<Body>, SearchError>>().await
        };
        let config = app.config.clone();
        start_search_task(&mut app, config, sender, &search).await;

        assert!(previous_cancelled.load(Ordering::SeqCst));
        assert!(app.search_task.is_some());

        cancel_search_task(&mut app).await;
        assert!(app.search_task.is_none());
    }
}
