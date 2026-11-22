use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use ed_planet_search::{
    application::{
        config::SearchConfig,
        search::{SearchError, SearchEvent, SearchUseCase},
    },
    cli, domain,
    presentation::tui,
    spansh::{SpanshClient, SpanshError},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Clap обрабатывает --help и --version до проверки TTY.
    // Аргументы командной строки становятся начальными значениями мастера,
    // но не обходят единый интерфейс настройки и запуска.
    let initial_config = if std::env::args_os().len() == 1 {
        None
    } else {
        Some(cli::parse_args()?)
    };

    ensure_interactive_terminal()?;
    tui::run(initial_config, run_search).await?;

    Ok(())
}

fn ensure_interactive_terminal() -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(anyhow!(
            "ED: Planet Search требует интерактивный терминал (stdin и stdout должны быть подключены к TTY)."
        ));
    }

    Ok(())
}

async fn run_search(
    config: SearchConfig,
    events: tokio::sync::mpsc::UnboundedSender<SearchEvent>,
) -> Result<Vec<domain::body::Body>, SearchError> {
    let _ = events.send(SearchEvent::ResolvingSystem);
    let client = SpanshClient::spansh(1).map_err(|error| {
        SearchError::Resolve(ed_planet_search::spansh::ResolveError::Spansh(
            SpanshError::Http(error),
        ))
    })?;
    let search = SearchUseCase::new(client);

    let center = search.resolve_center(&config.system).await?;
    let _ = events.send(SearchEvent::SystemResolved {
        name: center.name.clone(),
    });
    let progress = events.clone();
    let results = search
        .execute_with_progress(&center, &config.criteria, move |page, found| {
            let _ = progress.send(SearchEvent::PageLoaded { page, found });
        })
        .await?;
    Ok(results)
}
