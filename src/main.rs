mod cli;
#[allow(dead_code)]
mod domain;
mod parser;
#[allow(dead_code)]
mod spansh;

use cli::{Args, parse_range_f32, parse_range_i32};
use parser::{
    core::ParserCore,
    planet::parse_planet,
    search::{AtmosphereType, GravityRange, PlanetSearch, PlanetSearchParams, PlanetType},
};

use anyhow::{Result, anyhow};
use clap::Parser;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let planet_type = PlanetType::from_str(&args.planet_type)
        .ok_or_else(|| anyhow!("Неизвестный тип поверхности планеты: {}", args.planet_type))?;
    let atmosphere = AtmosphereType::from_str(&args.atmosphere_type)
        .ok_or_else(|| anyhow!("Неизвестный тип атмосферы: {}", args.atmosphere_type))?;

    let (gmin, gmax) = parse_range_f32(&args.gravity)?;
    let (pmin, pmax) = parse_range_f32(&args.pressure)?;
    let (tmin, tmax) = parse_range_i32(&args.temperature)?;

    let core = ParserCore::new()?;
    let params = PlanetSearchParams {
        commander_position: args.system.clone(),
        planet_type,
        atmosphere,
        gravity: GravityRange {
            min: gmin,
            max: gmax,
        },
        max_pages: 100,
    };

    let mut search = PlanetSearch::new(core.clone(), params);
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
    pb.enable_steady_tick(Duration::from_millis(125));

    while let Some(planets) = search.next_page().await? {
        pb.set_message(format!("Собираю данные о близлежащих системах... Найдено соответствий по заданным фильтрам: {}", results.len()));

        let unique: Vec<_> = planets
            .into_iter()
            .filter(|p| seen.insert(p.name.clone()))
            .collect();

        let parsed = stream::iter(unique)
            .map(|p| {
                let core = core.clone();
                async move { parse_planet(&core, &p.url, &p.name).await.ok() }
            })
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;

        for planet in parsed.into_iter().flatten() {
            let fits_f32 =
                |opt: Option<f32>, min: f32, max: f32| opt.is_some_and(|v| v >= min && v <= max);
            let fits_i32 =
                |opt: Option<i32>, min: i32, max: i32| opt.is_some_and(|v| v >= min && v <= max);

            if fits_f32(planet.gravity, gmin, gmax)
                && fits_f32(planet.pressure, pmin, pmax)
                && fits_i32(planet.surface_temp, tmin, tmax)
                && !planet.is_mapped
            {
                results.push(planet.clone());
                if results.len() >= args.count {
                    break;
                }
            }
        }

        if results.len() >= args.count {
            break;
        }
    }

    pb.finish_and_clear();

    if results.is_empty() {
        println!("❌ Не найдено подходящих планет.");
    } else {
        println!("\nСписок найденных планет:\n");
        for p in results {
            println!(
                "• {}\n  Температура: {:.1?}\n  Давление: {:.3?}\n  Гравитация: {:.3?}\n",
                p.name,
                p.surface_temp.unwrap_or_default(),
                p.pressure.unwrap_or_default(),
                p.gravity.unwrap_or_default()
            );
        }
    }

    Ok(())
}


