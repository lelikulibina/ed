use std::io::{self, Write};

use anyhow::Result;
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
                Some(Command::CopySystem(system)) => {
                    send_osc52(&system)?;
                    app.results_screen.set_status_message(format!(
                        "Скопировано: {system} · если OSC 52 не поддерживается, выделите имя вручную"
                    ));
                }
                None => {}
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

fn send_osc52(value: &str) -> Result<()> {
    let encoded = base64_encode(value.as_bytes());
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{encoded}\x07")?;
    stdout.flush()?;
    Ok(())
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).map_or(0, |value| u32::from(*value));
        let third = chunk.get(2).map_or(0, |value| u32::from(*value));
        let value = (first << 16) | (second << 8) | third;

        output.push(TABLE[((value >> 18) & 0x3f) as usize] as char);
        output.push(TABLE[((value >> 12) & 0x3f) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((value >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(value & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
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

    #[test]
    fn base64_encoding_handles_ascii_unicode_and_padding() {
        assert_eq!(base64_encode(b"Sol"), "U29s");
        assert_eq!(base64_encode("Колония".as_bytes()), "0JrQsNC70YzQvdC+0LLQvtC00L3QuNC5");
        assert_eq!(base64_encode(b"f"), "Zg==");
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
            Some(AppEvent::Search(SearchEvent::PageLoaded {
                page: 1,
                found: 0
            }))
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
