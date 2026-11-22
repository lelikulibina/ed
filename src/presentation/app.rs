use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;

use crate::{
    application::{config::SearchConfig, search::SearchEvent},
    domain::body::Body,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Search,
    Running,
    Results,
    Error,
}

pub enum AppEvent {
    Key(KeyEvent),
    Paste(String),
    Tick,
    Resize(Rect),
    Search(SearchEvent),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    StartSearch(SearchConfig),
    CancelSearch,
    CopySystem(String),
    Quit,
}

pub struct App {
    pub screen: Screen,
    pub config: SearchConfig,
    pub results: Vec<Body>,
    pub error_message: Option<String>,
    pub terminal_area: Rect,
    pub progress_message: Option<String>,
    pub current_page: u32,
    pub found_count: usize,
    pub selected_index: usize,
}

impl App {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            screen: Screen::Search,
            config,
            results: Vec::new(),
            error_message: None,
            terminal_area: Rect::default(),
            progress_message: None,
            current_page: 0,
            found_count: 0,
            selected_index: 0,
        }
    }
}

pub fn update(app: &mut App, event: AppEvent) -> Option<Command> {
    match event {
        AppEvent::Resize(area) => {
            app.terminal_area = area;
            None
        }
        AppEvent::Paste(_) | AppEvent::Tick => None,
        AppEvent::Search(event) => {
            match event {
                SearchEvent::ResolvingSystem => {
                    app.progress_message = Some("Поиск центральной системы…".to_owned());
                }
                SearchEvent::SystemResolved { name } => {
                    app.progress_message = Some(format!("Система найдена: {name}"));
                }
                SearchEvent::PageLoaded { page, found } => {
                    app.current_page = page;
                    app.found_count = found;
                    app.progress_message = Some(format!("Страница {page} · найдено {found}"));
                }
                SearchEvent::Finished(results) => {
                    app.results = results;
                    app.selected_index = 0;
                    app.progress_message = None;
                    app.error_message = None;
                    app.screen = Screen::Results;
                }
                SearchEvent::Failed(error) => {
                    app.progress_message = None;
                    app.error_message = Some(error.to_string());
                    app.screen = Screen::Error;
                }
            }
            None
        }
        AppEvent::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return None;
            }

            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(Command::Quit);
            }

            match app.screen {
                Screen::Search => match key.code {
                    KeyCode::Enter => {
                        app.screen = Screen::Running;
                        app.progress_message = Some("Подготовка поиска…".to_owned());
                        Some(Command::StartSearch(app.config.clone()))
                    }
                    KeyCode::Esc => Some(Command::Quit),
                    _ => None,
                },
                Screen::Running => match key.code {
                    KeyCode::Esc => {
                        app.screen = Screen::Search;
                        app.progress_message = None;
                        Some(Command::CancelSearch)
                    }
                    _ => None,
                },
                Screen::Results => match key.code {
                    KeyCode::Esc | KeyCode::Char('e') | KeyCode::Char('n') => {
                        app.screen = Screen::Search;
                        None
                    }
                    KeyCode::Up => {
                        app.selected_index = app.selected_index.saturating_sub(1);
                        None
                    }
                    KeyCode::Down => {
                        if app.selected_index + 1 < app.results.len() {
                            app.selected_index += 1;
                        }
                        None
                    }
                    KeyCode::Char('c') => app
                        .results
                        .get(app.selected_index)
                        .and_then(|body| body.system_name.clone())
                        .map(Command::CopySystem),
                    _ => None,
                },
                Screen::Error => match key.code {
                    KeyCode::Char('r') => {
                        app.screen = Screen::Running;
                        app.error_message = None;
                        app.progress_message = Some("Повторный запуск поиска…".to_owned());
                        Some(Command::StartSearch(app.config.clone()))
                    }
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e') => {
                        app.screen = Screen::Search;
                        app.error_message = None;
                        None
                    }
                    _ => None,
                },
            }
        }
    }
}

pub fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    use ratatui::widgets::{Block, Borders, Paragraph};

    let (title, message) = match app.screen {
        Screen::Search => ("ПОИСК ПЛАНЕТ", "Enter — начать поиск · Esc — выход"),
        Screen::Running => (
            "ПОИСК ВЫПОЛНЯЕТСЯ",
            app.progress_message.as_deref().unwrap_or("Ожидание событий…"),
        ),
        Screen::Results => ("РЕЗУЛЬТАТЫ", "↑↓ — выбор · c — копировать систему · n — новый поиск"),
        Screen::Error => (
            "ОШИБКА ПОИСКА",
            app.error_message.as_deref().unwrap_or("Неизвестная ошибка"),
        ),
    };
    let area = frame.area();
    frame.render_widget(
        Paragraph::new(message).block(Block::default().title(title).borders(Borders::ALL)),
        area,
    );
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::config::SearchConfig,
        domain::criteria::{Filter, NumericRange, SearchCriteria},
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn app() -> App {
        let criteria = SearchCriteria::new(
            Filter::Any,
            Filter::Any,
            NumericRange {
                min: 0.0,
                max: 5_000.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.0,
                max: 10.0,
            },
            5,
        )
        .unwrap();
        App::new(SearchConfig::new("Sol", criteria).unwrap())
    }

    #[test]
    fn enter_starts_search_and_moves_to_running_screen() {
        let mut app = app();
        let expected = Command::StartSearch(app.config.clone());

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        );

        assert_eq!(command, Some(expected));
        assert_eq!(app.screen, Screen::Running);
    }

    #[test]
    fn escape_cancels_running_search_and_returns_to_search_screen() {
        let mut app = app();
        app.screen = Screen::Running;

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        );

        assert_eq!(command, Some(Command::CancelSearch));
        assert_eq!(app.screen, Screen::Search);
    }

    #[test]
    fn finished_search_moves_to_results_and_keeps_bodies() {
        let mut app = app();
        app.screen = Screen::Running;
        let bodies = vec![Body::from_json(&serde_json::json!({"name": "Earth"})).unwrap()];

        let command = update(
            &mut app,
            AppEvent::Search(SearchEvent::Finished(bodies.clone())),
        );

        assert!(command.is_none());
        assert_eq!(app.screen, Screen::Results);
        assert_eq!(app.results, bodies);
    }

    #[test]
    fn error_screen_can_retry_the_previous_configuration() {
        let mut app = app();
        app.screen = Screen::Error;
        app.error_message = Some("Сбой сети".to_owned());

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        );

        assert_eq!(command, Some(Command::StartSearch(app.config.clone())));
        assert_eq!(app.screen, Screen::Running);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn results_screen_copies_the_selected_system_name() {
        let mut app = app();
        app.screen = Screen::Results;
        app.results = vec![
            Body::from_json(&serde_json::json!({
                "name": "Earth",
                "system": {"name": "Sol"}
            }))
            .unwrap(),
        ];

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
        );

        assert_eq!(command, Some(Command::CopySystem("Sol".to_owned())));
    }

    #[test]
    fn resize_updates_only_the_terminal_area() {
        let mut app = app();
        let area = Rect::new(0, 0, 120, 40);

        let command = update(&mut app, AppEvent::Resize(area));

        assert!(command.is_none());
        assert_eq!(app.terminal_area, area);
        assert_eq!(app.screen, Screen::Search);
    }

    #[test]
    fn renderer_displays_the_current_screen_without_mutating_state() {
        let app = app();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("ПОИСК ПЛАНЕТ"));
        assert_eq!(app.screen, Screen::Search);
    }

    #[test]
    fn failed_search_moves_to_error_screen_with_message() {
        let mut app = app();
        app.screen = Screen::Running;

        let command = update(
            &mut app,
            AppEvent::Search(SearchEvent::Failed(
                crate::application::search::SearchError::Resolve(
                    crate::spansh::ResolveError::NotFound("Sol".to_owned()),
                ),
            )),
        );

        assert!(command.is_none());
        assert_eq!(app.screen, Screen::Error);
        assert!(
            app.error_message
                .as_deref()
                .is_some_and(|message| !message.is_empty())
        );
    }
}
