use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use tokio::{sync::mpsc, task::JoinHandle};

use super::app::AppEvent;

pub fn spawn_event_stream(sender: mpsc::Sender<AppEvent>) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        loop {
            let event = match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(event) => to_app_event(event),
                    Err(_) => break,
                },
                Ok(false) => Some(AppEvent::Tick),
                Err(_) => break,
            };

            if let Some(event) = event
                && sender.blocking_send(event).is_err()
            {
                break;
            }
        }
    })
}

fn to_app_event(event: Event) -> Option<AppEvent> {
    match event {
        Event::Key(key) => Some(AppEvent::Key(key)),
        Event::Paste(text) => Some(AppEvent::Paste(text)),
        Event::Resize(width, height) => Some(AppEvent::Resize(Rect::new(0, 0, width, height))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn converts_terminal_input_into_application_events() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(
            to_app_event(Event::Key(key)),
            Some(AppEvent::Key(_))
        ));
        assert!(matches!(
            to_app_event(Event::Paste("Sol".to_owned())),
            Some(AppEvent::Paste(text)) if text == "Sol"
        ));
        assert!(matches!(
            to_app_event(Event::Resize(120, 40)),
            Some(AppEvent::Resize(Rect {
                width: 120,
                height: 40,
                ..
            }))
        ));
        assert!(to_app_event(Event::FocusGained).is_none());
    }
}
