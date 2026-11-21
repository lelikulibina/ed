mod application;
mod cli;
mod domain;
mod presentation;
mod spansh;

use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use clap::Parser;

use application::{config::SearchConfig, search::SearchUseCase};
use cli::Args;
use presentation::tui;
use spansh::SpanshClient;

#[tokio::main]
async fn main() -> Result<()> {
    let config = if std::env::args_os().len() == 1 {
        ensure_interactive_terminal()?;
        let Some(config) = tui::run_tui()? else {
            return Ok(());
        };
        config
    } else {
        // Clap обрабатывает --help и --version до проверки TTY,
        // поэтому справка доступна и в неинтерактивном окружении.
        let config = Args::parse().into_search_config()?;
        ensure_interactive_terminal()?;
        config
    };

    // Оба способа задания критериев сходятся в одной точке:
    // поиск и отображение результатов всегда проходят через TUI.
    tui::run_search_ui(async move { run_search(config).await }).await
}

fn ensure_interactive_terminal() -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(anyhow!(
            "ED: Planet Search требует интерактивный терминал (stdin и stdout должны быть подключены к TTY)."
        ));
    }

    Ok(())
}

async fn run_search(config: SearchConfig) -> Result<Vec<domain::body::Body>> {
    let client = SpanshClient::spansh(1)?;
    let search = SearchUseCase::new(client);

    let center = search.resolve_center(&config.system).await?;
    let results = search.execute(&center, &config.criteria).await?;
    Ok(results)
}
