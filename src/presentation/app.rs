use crossterm::event::KeyEvent;
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
}

impl App {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            screen: Screen::Search,
            config,
            results: Vec::new(),
            error_message: None,
            terminal_area: Rect::default(),
        }
    }
}

pub fn update(_app: &mut App, _event: AppEvent) -> Option<Command> {
    None
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
            NumericRange { min: 0.0, max: 5_000.0 },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange { min: 0.0, max: 10.0 },
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
        let bodies = vec![Body::from_json(&serde_json::json!({"name":"Earth"})).unwrap()];

        let command = update(&mut app, AppEvent::Search(SearchEvent::Finished(bodies.clone())));

        assert!(command.is_none());
        assert_eq!(app.screen, Screen::Results);
        assert_eq!(app.results, bodies);
    }

    #[test]
    fn failed_search_moves_to_error_screen_with_message() {
        let mut app = app();
        app.screen = Screen::Running;

        let command = update(
            &mut app,
            AppEvent::Search(SearchEvent::Failed(
                crate::application::search::SearchError::Resolve(
                    crate::spansh::ResolveError::NoCandidates("Sol".to_owned()),
                ),
            )),
        );

        assert!(command.is_none());
        assert_eq!(app.screen, Screen::Error);
        assert!(app.error_message.as_deref().is_some_and(|message| !message.is_empty()));
    }
}
