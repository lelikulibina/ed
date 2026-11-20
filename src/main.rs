mod application;
mod cli;
mod domain;
mod presentation;
mod spansh;

use anyhow::{Result, anyhow};
use clap::{Parser, error::ErrorKind};
use std::io;

use application::search::SearchUseCase;
use cli::{Args, parse_range_f32, parse_range_i32};
use domain::criteria::{NumericRange, SearchCriteria};
use presentation::{
    menu::{MenuAction, run_menu},
    output::print_results,
};
use spansh::SpanshClient;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args_os().len() == 1 {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut output = io::stdout().lock();
        return match run_menu(&mut input, &mut output)? {
            MenuAction::Exit => Ok(()),
            MenuAction::Search { system, criteria } => run_search(&system, &criteria).await,
        };
    }

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let criteria = criteria_from_args(&args)?;
    run_search(&args.system, &criteria).await
}

fn criteria_from_args(args: &Args) -> Result<SearchCriteria> {
    let (tmin, tmax) = parse_range_i32(&args.temperature)?;
    let (gmin, gmax) = parse_range_f32(&args.gravity)?;
    let (pmin, pmax) = parse_range_f32(&args.pressure)?;
    let temperature = NumericRange::new(f64::from(tmin), f64::from(tmax))
        .ok_or_else(|| anyhow!("Некорректный диапазон температуры: {}", args.temperature))?;
    let gravity = NumericRange::new(f64::from(gmin), f64::from(gmax))
        .ok_or_else(|| anyhow!("Некорректный диапазон гравитации: {}", args.gravity))?;
    let pressure = NumericRange::new(f64::from(pmin), f64::from(pmax))
        .ok_or_else(|| anyhow!("Некорректный диапазон давления: {}", args.pressure))?;
    Ok(SearchCriteria::from_cli(
        &args.planet_type,
        &args.atmosphere_type,
        temperature,
        gravity,
        pressure,
        args.count,
    )?
    .with_bio_signal_exclusion(args.exclude_with_bio_signals))
}

async fn run_search(system: &str, criteria: &SearchCriteria) -> Result<()> {
    if !criteria.volcanism.is_supported() {
        return Err(anyhow!(
            "Условие вулканизма «{}» пока нельзя надёжно проверить по данным Spansh; поиск не запущен.",
            criteria.volcanism.description()
        ));
    }
    let client = SpanshClient::spansh(1)?;
    let search = SearchUseCase::new(client);
    println!("Разрешаю центральную систему «{}»...", system);
    let center = search.resolve_center(system).await?;
    if !center.name.eq_ignore_ascii_case(system.trim()) {
        println!(
            "ВНИМАНИЕ: точное совпадение «{}» не найдено в Spansh. Используется ближайшее совпадение из ответа API: «{}».",
            system, center.name
        );
    }
    println!(
        "Центр поиска: {} (ID64: {}), координаты: ({:.3}, {:.3}, {:.3})",
        center.name, center.id64, center.coordinates.x, center.coordinates.y, center.coordinates.z
    );
    println!("Ищу подходящие планеты в радиусе 1000 св. л....");
    let results = search.execute(&center, criteria).await?;
    print_results(&results);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_arguments_keep_existing_defaults_and_help_is_recognized() {
        let args =
            Args::try_parse_from(["ed", "-S", "Sol", "-P", "rocky body", "-A", "water"]).unwrap();
        let criteria = criteria_from_args(&args).unwrap();
        assert_eq!(criteria.count, 5);
        assert!((criteria.gravity.min - 0.1).abs() < 1e-6);
        assert_eq!(criteria.gravity.max, 1.0);
        assert_eq!(
            criteria.temperature,
            NumericRange {
                min: 0.0,
                max: 5000.0
            }
        );
        assert_eq!(
            criteria.pressure,
            NumericRange {
                min: 0.0,
                max: 10.0
            }
        );
        assert!(matches!(
            Args::try_parse_from(["ed", "--help"]).unwrap_err().kind(),
            ErrorKind::DisplayHelp
        ));
        assert!(matches!(
            Args::try_parse_from(["ed", "--version"])
                .unwrap_err()
                .kind(),
            ErrorKind::DisplayVersion
        ));
    }
}
