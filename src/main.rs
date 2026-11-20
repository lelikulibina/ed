mod application;
mod cli;
mod domain;
mod presentation;
mod spansh;

use anyhow::{Result, anyhow};
use clap::Parser;

use application::search::SearchUseCase;
use cli::{Args, parse_range_f32, parse_range_i32};
use domain::criteria::{NumericRange, SearchCriteria};
use presentation::output::print_results;
use spansh::SpanshClient;

#[tokio::main]
async fn main() -> Result<()> {
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
    )?;

    let client = SpanshClient::spansh(1)?;
    let search = SearchUseCase::new(client);

    println!("Разрешаю центральную систему «{}»...", args.system);
    let center = search.resolve_center(&args.system).await?;
    if !center.name.eq_ignore_ascii_case(args.system.trim()) {
        println!(
            "ВНИМАНИЕ: точное совпадение «{}» не найдено в Spansh. Используется ближайшее совпадение из ответа API: «{}».",
            args.system, center.name
        );
    }
    println!(
        "Центр поиска: {} (ID64: {}), координаты: ({:.3}, {:.3}, {:.3})",
        center.name, center.id64, center.coordinates.x, center.coordinates.y, center.coordinates.z
    );

    println!("Ищу подходящие планеты в радиусе 1000 св. л....");
    let results = search.execute(&center, &criteria).await?;
    print_results(&results);

    Ok(())
}
