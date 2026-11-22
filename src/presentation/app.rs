use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    application::{config::SearchConfig, search::SearchEvent},
    domain::body::Body,
};

use super::text::ui;

use super::screens::{
    results::{ResultsAction, ResultsScreen},
    search::{SearchScreen, SearchScreenAction},
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
    pub resolved_system: Option<String>,
    pub selected_index: usize,
    pub search_screen: SearchScreen,
    pub results_screen: ResultsScreen,
    pub search_task: Option<tokio::task::JoinHandle<()>>,
    pub help_open: bool,
}

impl App {
    pub fn new(config: SearchConfig) -> Self {
        let search_screen = SearchScreen::from_config(&config);
        Self::with_search_screen(config, search_screen)
    }

    pub fn new_default() -> Self {
        let search_screen = SearchScreen::default();
        let config = search_screen
            .form
            .build()
            .expect(ui::APP_DEFAULT_CONFIG_ERROR);
        Self::with_search_screen(config, search_screen)
    }

    fn with_search_screen(config: SearchConfig, search_screen: SearchScreen) -> Self {
        Self {
            screen: Screen::Search,
            config,
            results: Vec::new(),
            error_message: None,
            terminal_area: Rect::default(),
            progress_message: None,
            current_page: 0,
            found_count: 0,
            resolved_system: None,
            selected_index: 0,
            search_screen,
            results_screen: ResultsScreen::default(),
            search_task: None,
            help_open: false,
        }
    }
}

fn is_quit_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q' | 'Q' | 'й' | 'Й'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub fn update(app: &mut App, event: AppEvent) -> Option<Command> {
    match event {
        AppEvent::Resize(area) => {
            app.terminal_area = area;
            None
        }
        AppEvent::Paste(text) => {
            if app.screen == Screen::Search {
                app.search_screen.handle_paste(&text);
            }
            None
        }
        AppEvent::Tick => None,
        AppEvent::Search(event) => {
            match event {
                SearchEvent::ResolvingSystem => {
                    app.resolved_system = None;
                    app.current_page = 0;
                    app.found_count = 0;
                    app.progress_message = Some(ui::APP_SEARCH_RESOLVING_SYSTEM.to_owned());
                }
                SearchEvent::SystemResolved { name } => {
                    app.resolved_system = Some(name.clone());
                    app.progress_message =
                        Some(format!("{}: {name}", ui::APP_SEARCH_SYSTEM_FOUND_PREFIX));
                }
                SearchEvent::PageLoaded { page, found } => {
                    app.current_page = page;
                    app.found_count = found;
                    app.progress_message = Some(format!(
                        "{}: {page} · {} {found}",
                        ui::APP_SEARCH_PAGE_LOADED_PREFIX,
                        ui::APP_SEARCH_PAGE_FOUND_SUFFIX
                    ));
                }
                SearchEvent::Finished(results) => {
                    app.results_screen.reset(&results);
                    app.results = results;
                    app.selected_index = 0;
                    app.progress_message = None;
                    app.error_message = None;
                    app.screen = Screen::Results;
                }
                SearchEvent::Failed(error) => {
                    app.progress_message = None;
                    app.error_message = Some(error.user_message());
                    app.screen = Screen::Error;
                }
            }
            None
        }
        AppEvent::Key(key) => {
            if app.help_open {
                if key.kind == KeyEventKind::Press && is_quit_key(key) {
                    return Some(Command::Quit);
                }
                if key.kind == KeyEventKind::Press
                    && (key.code == KeyCode::Esc || key.code == KeyCode::Char('?'))
                {
                    app.help_open = false;
                }
                return None;
            }

            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('?') {
                app.help_open = true;
                return None;
            }

            if key.code == KeyCode::Esc {
                return match app.screen {
                    Screen::Search => Some(Command::Quit),
                    Screen::Running => {
                        app.screen = Screen::Search;
                        app.progress_message = None;
                        Some(Command::CancelSearch)
                    }
                    Screen::Results => {
                        app.screen = Screen::Search;
                        None
                    }
                    Screen::Error => {
                        app.screen = Screen::Search;
                        app.error_message = None;
                        None
                    }
                };
            }

            if is_quit_key(key) {
                return Some(Command::Quit);
            }

            if key.kind != KeyEventKind::Press {
                return None;
            }

            match app.screen {
                Screen::Search => match app.search_screen.handle_key(key) {
                    SearchScreenAction::Start(config) => {
                        app.config = config.clone();
                        app.screen = Screen::Running;
                        app.progress_message = Some(ui::APP_SEARCH_PREPARING.to_owned());
                        Some(Command::StartSearch(config))
                    }
                    SearchScreenAction::Exit => Some(Command::Quit),
                    SearchScreenAction::None => None,
                },
                Screen::Running => match key.code {
                    KeyCode::Esc => {
                        app.screen = Screen::Search;
                        app.progress_message = None;
                        Some(Command::CancelSearch)
                    }
                    _ => None,
                },
                Screen::Results => match app.results_screen.handle_key(key, &app.results) {
                    Some(ResultsAction::BackToSearch) => {
                        app.screen = Screen::Search;
                        None
                    }
                    Some(ResultsAction::CopySystem(system)) => Some(Command::CopySystem(system)),
                    None => None,
                },
                Screen::Error => match key.code {
                    KeyCode::Char('r' | 'R' | 'к' | 'К') => {
                        app.screen = Screen::Running;
                        app.error_message = None;
                        app.progress_message = Some(ui::APP_SEARCH_RETRYING.to_owned());
                        Some(Command::StartSearch(app.config.clone()))
                    }
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('e' | 'E' | 'у' | 'У') => {
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
    match app.screen {
        Screen::Search => super::screens::search::render(frame, &app.search_screen),
        Screen::Running => super::screens::running::render(frame, app),
        Screen::Error => super::screens::error::render(frame, app),
        Screen::Results => super::screens::results::render(frame, app),
    }

    if app.help_open {
        let help = match app.screen {
            Screen::Search => ui::HELP_SEARCH,
            Screen::Running => ui::HELP_RUNNING,
            Screen::Results => ui::HELP_RESULTS,
            Screen::Error => ui::HELP_ERROR,
        };
        let content = format!("{help}\n\n{}", ui::HELP_COMMON);
        let area = super::theme::centered(
            frame.area(),
            frame.area().width.saturating_sub(4),
            frame.area().height.saturating_sub(4),
        );
        frame.render_widget(
            Paragraph::new(content).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(ui::HELP_TITLE)
                    .borders(Borders::ALL)
                    .border_style(super::theme::Theme::default().accent),
            ),
            area,
        );
    }
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
    fn empty_search_results_still_open_the_results_screen() {
        let mut app = app();
        app.screen = Screen::Running;

        update(
            &mut app,
            AppEvent::Search(SearchEvent::Finished(Vec::new())),
        );

        assert_eq!(app.screen, Screen::Results);
        assert!(app.results.is_empty());
    }

    #[test]
    fn error_screen_can_return_to_search_and_keep_the_configuration() {
        let mut app = app();
        app.screen = Screen::Error;
        app.error_message = Some("Система не найдена".to_owned());
        let expected = app.config.clone();

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        );

        assert!(command.is_none());
        assert_eq!(app.screen, Screen::Search);
        assert_eq!(app.config, expected);
        assert!(app.error_message.is_none());
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
        assert!(rendered.contains("ED Planet Search"));
        assert_eq!(app.screen, Screen::Search);
    }

    #[test]
    fn progress_events_update_the_running_screen_in_order() {
        let mut app = app();

        update(&mut app, AppEvent::Search(SearchEvent::ResolvingSystem));
        assert_eq!(
            app.progress_message.as_deref(),
            Some("Поиск центральной системы (Spansh)…")
        );

        update(
            &mut app,
            AppEvent::Search(SearchEvent::SystemResolved {
                name: "Sol".to_owned(),
            }),
        );
        assert_eq!(app.resolved_system.as_deref(), Some("Sol"));
        assert_eq!(
            app.progress_message.as_deref(),
            Some("Система найдена: Sol")
        );

        update(
            &mut app,
            AppEvent::Search(SearchEvent::PageLoaded { page: 3, found: 2 }),
        );
        assert_eq!(app.current_page, 3);
        assert_eq!(app.found_count, 2);
        assert_eq!(
            app.progress_message.as_deref(),
            Some("Страница Spansh: 3 · найдено 2")
        );
    }

    #[test]
    fn running_and_error_screens_render_actionable_content() {
        let mut app = app();
        app.screen = Screen::Running;
        app.resolved_system = Some("Sol".to_owned());
        app.current_page = 2;
        app.found_count = 3;

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let running = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(running.contains("✓"));
        assert!(running.contains("Центральная система найдена"));
        assert!(running.contains("Загружена страница Spansh: 2"));
        assert!(running.contains("Подходящих планет найдено: 3 из 5"));

        app.screen = Screen::Error;
        app.error_message = Some("Система не найдена".to_owned());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let error = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(error.contains("✗"));
        assert!(error.contains("Система не найдена"));
        assert!(error.contains("r / к — повторить поиск"));
        assert!(error.contains("e / у / Enter / Esc"));
        assert!(error.contains("Ctrl+Q / Ctrl+Й — выход"));
    }

    #[test]
    fn ctrl_y_quits_from_search_and_help_overlay() {
        let mut app = app();
        let ctrl_y = KeyEvent::new(KeyCode::Char('й'), KeyModifiers::CONTROL);

        assert_eq!(update(&mut app, AppEvent::Key(ctrl_y)), Some(Command::Quit));

        app.help_open = true;
        assert_eq!(update(&mut app, AppEvent::Key(ctrl_y)), Some(Command::Quit));
    }

    #[test]
    fn error_screen_shows_resize_message_below_minimum_size() {
        let mut app = app();
        app.screen = Screen::Error;
        app.error_message = Some("Сбой сети".to_owned());

        let backend = ratatui::backend::TestBackend::new(60, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Увеличьте окно терминала до 80×24"));
        assert!(!rendered.contains("Сбой сети"));
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
                .is_some_and(|message| message.contains("не найдена"))
        );
    }

    #[test]
    fn question_mark_opens_global_help_on_every_screen() {
        for screen in [
            Screen::Search,
            Screen::Running,
            Screen::Results,
            Screen::Error,
        ] {
            let mut app = app();
            app.screen = screen;
            let command = update(
                &mut app,
                AppEvent::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE)),
            );
            assert!(command.is_none());
            assert!(app.help_open, "help should open on {screen:?}");
            assert_eq!(app.screen, screen);
        }
    }

    #[test]
    fn global_help_closes_without_changing_the_current_screen() {
        let mut app = app();
        app.screen = Screen::Running;
        app.help_open = true;

        let command = update(
            &mut app,
            AppEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        );

        assert!(command.is_none());
        assert!(!app.help_open);
        assert_eq!(app.screen, Screen::Running);
    }

    #[test]
    fn global_help_overlay_renders_running_screen_shortcuts() {
        let mut app = app();
        app.screen = Screen::Running;
        app.help_open = true;
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
        assert!(rendered.contains("ВЫПОЛНЕНИЕ"));
        assert!(rendered.contains("Esc — отменить поиск"));
        assert!(rendered.contains("Ctrl+Q — выход"));
    }
}
