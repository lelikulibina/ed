use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use tokio::{sync::mpsc, task::JoinHandle};

use super::app::AppEvent;

pub fn poll_and_send(
    sender: &mpsc::Sender<AppEvent>,
    timeout: Duration,
    animation_enabled: bool,
) -> anyhow::Result<bool> {
    if event::poll(timeout)? {
        if let Some(event) = to_app_event(event::read()?) {
            sender
                .try_send(event)
                .map_err(|error| anyhow::anyhow!("Не удалось передать событие интерфейсу: {error}"))?;
            return Ok(true);
        }
    } else if animation_enabled {
        sender
            .try_send(AppEvent::Tick)
            .map_err(|error| anyhow::anyhow!("Не удалось передать событие анимации: {error}"))?;
        return Ok(true);
    }

    Ok(false)
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
