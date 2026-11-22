use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use ed_planet_search::{
    application::{
        config::SearchConfig,
        search::{SearchError, SearchEvent, SearchUseCase},
    },
    cli, domain,
    presentation::runtime,
    spansh::{SpanshClient, SpanshError},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Clap обрабатывает --help и --version до проверки TTY.
    // CLI-конфигурация запускается сразу; без аргументов открывается форма поиска.
    let initial_config = if std::env::args_os().len() == 1 {
        None
    } else {
        Some(cli::parse_args()?)
    };

    ensure_interactive_terminal()?;
    runtime::run(initial_config, run_search).await?;

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
    let client = SpanshClient::spansh(1).map_err(|error| {
        SearchError::Resolve(ed_planet_search::spansh::ResolveError::Spansh(
            SpanshError::Http(error),
        ))
    })?;
    SearchUseCase::new(client)
        .search_with_events(&config.system, &config.criteria, events)
        .await
}
