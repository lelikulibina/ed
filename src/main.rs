mod application;
mod cli;
mod domain;
mod presentation;
mod spansh;

use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use clap::Parser;

use application::{
    config::SearchConfig,
    search::SearchUseCase,
};
use cli::{Args, parse_range_f32, parse_range_i32};
use domain::criteria::{NumericRange, SearchCriteria};
use presentation::tui;
use spansh::SpanshClient;

#[tokio::main]
async fn main() -> Result<()> {
    let config = if std::env::args_os().len() == 1 {
        if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
            return Err(anyhow!("Для работы ED: Planet Search требуется интерактивный терминал."));
        }
        let Some(selection) = tui::run_tui()? else {
            return Ok(());
        };
        SearchConfig::new(selection.system, selection.criteria)?
    } else {
        let args = Args::parse();
        let (temperature_min, temperature_max) = parse_range_i32(&args.temperature)?;
        let (gravity_min, gravity_max) = parse_range_f32(&args.gravity)?;
        let (pressure_min, pressure_max) = parse_range_f32(&args.pressure)?;

        let temperature = NumericRange::new(f64::from(temperature_min), f64::from(temperature_max))
            .ok_or_else(|| anyhow!("Некорректный диапазон температуры: {}", args.temperature))?;
        let gravity = NumericRange::new(f64::from(gravity_min), f64::from(gravity_max))
            .ok_or_else(|| anyhow!("Некорректный диапазон гравитации: {}", args.gravity))?;
        let pressure = NumericRange::new(f64::from(pressure_min), f64::from(pressure_max))
            .ok_or_else(|| anyhow!("Некорректный диапазон давления: {}", args.pressure))?;

        let criteria = SearchCriteria::from_cli(
            &args.planet_type,
            &args.atmosphere_type,
            temperature,
            gravity,
            pressure,
            args.count,
        )?
        .with_bio_signal_exclusion(args.exclude_with_bio_signals);
        SearchConfig::new(args.system, criteria)?
    };

    if !std::io::stdout().is_terminal() {
        return Err(anyhow!("Для отображения результатов требуется интерактивный терминал."));
    }

    let results = run_search(&config).await?;
    tui::show_results(&results)
}

async fn run_search(config: &SearchConfig) -> Result<Vec<domain::body::Body>> {
    let client = SpanshClient::spansh(1)?;
    let search = SearchUseCase::new(client);

    let center = search.resolve_center(&config.system).await?;
    let results = search.execute(&center, &config.criteria).await?;
    Ok(results)
}
