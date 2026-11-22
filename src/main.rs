use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use ed_planet_search::{
    application::{config::SearchConfig, search::SearchUseCase},
    cli,
    domain,
    presentation::tui,
    spansh::SpanshClient,
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
    let mut initial_config = initial_config;

    loop {
        let Some(config) = tui::run_tui(initial_config.take())? else {
            break;
        };
        let previous_config = config.clone();

        match tui::run_search_ui(async move { run_search(config).await }).await? {
            tui::SearchUiAction::BackToSetup => {
                initial_config = Some(previous_config);
            }
            tui::SearchUiAction::Exit => break,
        }
    }

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

async fn run_search(config: SearchConfig) -> Result<Vec<domain::body::Body>> {
    let client = SpanshClient::spansh(1)?;
    let search = SearchUseCase::new(client);

    let center = search.resolve_center(&config.system).await?;
    let results = search.execute(&center, &config.criteria).await?;
    Ok(results)
}
