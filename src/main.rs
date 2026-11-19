mod parser;

use parser::{
    core::ParserCore,
    search::{AtmosphereType, GravityRange, PlanetSearch, PlanetSearchParams, PlanetType},
};

use anyhow::{Result, anyhow};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "ED:Planet Search",
    version,
    about = "Поисковик планет в базе EDSM (с расширеной фильтрацией).\n\
        Позволяет искать планеты по типу поверхности, атмосфере и физическим параметрам"
)]
struct Args {
    #[arg(
        short = 'S',
        long,
        help = "Название опорной звёздной системы, например: \"Col 285 Sector BQ-Y d40\". Поиск выполняется в радиусе 100 св. лет."
    )]
    system: String,

    #[arg(
        short = 'P',
        long = "planet-type",
        help = "Тип поверхности планеты. Допустимые значения:\n\
      - rocky body             — каменная планета\n\
      - icy body               — ледяная планета\n\
      - rocky ice world        — каменно-ледяной мир\n\
      - high metal content world — планета с высоким содержанием металлов\n\
      - any                    — не фильтровать по типу поверхности"
    )]
    planet_type: String,

    #[arg(
        short = 'A',
        long = "atmosphere-type",
        help = "Тип атмосферы планеты. Допустимые значения:\n\
      - ammonia          — аммиачная\n\
      - water            — водяная\n\
      - carbon-dioxide   — углекислотная\n\
      - sulphur-dioxide  — сернистая\n\
      - nitrogen         — азотная\n\
      - methane          — метановая\n\
      - helium           — гелиевая\n\
      - silicate vapour  — силикатный пар\n\
      - neon             — неоновая\n\
      - argon            — аргоновая\n\
      - oxygen           — кислородная\n\
      - any              — не фильтровать по типу атмосферы"
    )]
    atmosphere_type: String,

    #[arg(
        short = 't',
        long,
        default_value = "0-5000",
        help = "Диапазон температуры поверхности планеты (int). Формат: min-max"
    )]
    temperature: String,

    #[arg(
        short = 'g',
        long,
        default_value = "0.1-1.0",
        help = "Диапазон гравитации на планете (float). Формат: min-max"
    )]
    gravity: String,

    #[arg(
        short = 'p',
        long,
        default_value = "0.0-10.0",
        help = "Диапазон давления на планете (float). Формат: min-max"
    )]
    pressure: String,

    #[arg(
        short = 'c',
        long,
        default_value_t = 5,
        help = "Количество планет, которые необходимо найти"
    )]
    count: usize,
}

fn parse_range_f32(s: &str) -> Result<(f32, f32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        Err(anyhow!("Неверный диапазон: {}", s))
    } else {
        Ok((parts[0].parse()?, parts[1].parse()?))
    }
}

fn parse_range_i32(s: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        Err(anyhow!("Неверный диапазон: {}", s))
    } else {
        Ok((parts[0].parse()?, parts[1].parse()?))
    }
}

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
    let mut mapping_status_unknown = false;

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
    pb.enable_steady_tick(Duration::from_millis(125));

    while let Some(planets) = search.next_page().await? {
        pb.set_message(format!(
            "Проверяю системы через EDSM API... Найдено соответствий по заданным фильтрам: {}",
            results.len()
        ));

        for planet in planets {
            if !seen.insert(planet.name.clone()) {
                continue;
            }

            if planet.is_mapped == Some(true) {
                continue;
            }
            if planet.is_mapped.is_none() {
                mapping_status_unknown = true;
            }

            let fits_f32 =
                |opt: Option<f32>, min: f32, max: f32| opt.is_some_and(|v| v >= min && v <= max);
            let fits_i32 =
                |opt: Option<i32>, min: i32, max: i32| opt.is_some_and(|v| v >= min && v <= max);

            if fits_f32(planet.gravity, gmin, gmax)
                && fits_f32(planet.pressure, pmin, pmax)
                && fits_i32(planet.surface_temp, tmin, tmax)
            {
                results.push(planet);
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
        println!("❌ Не найдено подходящих планет в проверенных системах.");
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

    if mapping_status_unknown {
        eprintln!(
            "⚠️ EDSM API не предоставляет подтверждённый статус картографирования планет. Планеты с неизвестным статусом включены в результаты; проверьте их в игре перед посещением."
        );
    }

    Ok(())
}
