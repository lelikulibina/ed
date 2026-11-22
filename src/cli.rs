use anyhow::{Result, anyhow};
use clap::Parser;

use crate::{
    application::{config::SearchConfig, form::SearchForm},
    domain::catalog::{AtmosphereType, SurfaceType},
};

#[derive(Parser, Debug)]
#[command(
    name = "ED:Planet Search",
    version,
    about = "Поисковик планет в базе Spansh (с расширенной фильтрацией).\n\
        Позволяет искать планеты по типу поверхности, атмосфере и физическим параметрам"
)]
pub(crate) struct Args {
    #[arg(
        short = 'S',
        long,
        help = "Название центральной системы, вокруг которой будет выполняться поиск. Например: \"Col 285 Sector BQ-Y d40\""
    )]
    pub(crate) system: String,

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
    pub(crate) planet_type: String,

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
    pub(crate) atmosphere_type: String,

    #[arg(
        short = 't',
        long,
        default_value = "0-5000",
        help = "Диапазон температуры поверхности планеты (K). Формат: min-max"
    )]
    pub(crate) temperature: String,

    #[arg(
        short = 'g',
        long,
        default_value = "0.1-1.0",
        help = "Диапазон гравитации на планете (float). Формат: min-max"
    )]
    pub(crate) gravity: String,

    #[arg(
        short = 'p',
        long,
        default_value = "0.0-10.0",
        help = "Диапазон давления на планете (float). Формат: min-max"
    )]
    pub(crate) pressure: String,

    #[arg(
        short = 'c',
        long,
        default_value_t = 5,
        help = "Количество планет, которые необходимо найти"
    )]
    pub(crate) count: usize,

    #[arg(
        long = "exclude-with-bio-signals",
        default_value_t = false,
        help = "Исключать планеты, у которых Spansh зарегистрировал хотя бы один биологический сигнал или указал биологические группы"
    )]
    pub(crate) exclude_with_bio_signals: bool,
}

impl Args {
    /// Преобразует аргументы командной строки в ту же конфигурацию,
    /// которую использует интерактивный интерфейс.
    pub(crate) fn into_search_config(self) -> Result<SearchConfig> {
        let form = SearchForm {
            system: self.system,
            surface_types: vec![self.planet_type.parse::<SurfaceType>()?],
            atmosphere_types: vec![self.atmosphere_type.parse::<AtmosphereType>()?],
            temperature: self.temperature,
            gravity: self.gravity,
            pressure: self.pressure,
            count: self.count.to_string(),
            exclude_with_bio_signals: self.exclude_with_bio_signals,
        };

        form.build().map_err(|errors| {
            anyhow!(
                "{}",
                errors
                    .into_iter()
                    .map(|error| error.message)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    }
}

pub fn parse_args() -> Result<SearchConfig> {
    Args::parse().into_search_config()
}

pub(crate) fn parse_range_f64(s: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        Err(anyhow!("Неверный диапазон: {}", s))
    } else {
        Ok((parts[0].parse()?, parts[1].parse()?))
    }
}

pub(crate) fn parse_range_i32(s: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        Err(anyhow!("Неверный диапазон: {}", s))
    } else {
        Ok((parts[0].parse()?, parts[1].parse()?))
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use crate::domain::{
        catalog::{AtmosphereType, SurfaceType},
        criteria::Filter,
    };
    use clap::Parser;

    #[test]
    fn parses_legacy_cli_flag_variants_and_defaults() {
        struct Case {
            name: &'static str,
            arguments: Vec<&'static str>,
            planet_type: &'static str,
            atmosphere_type: &'static str,
            temperature: &'static str,
            gravity: &'static str,
            pressure: &'static str,
            count: usize,
            exclude_with_bio_signals: bool,
        }

        let cases = [
            Case {
                name: "long flags with defaults",
                arguments: vec![
                    "ed_planet_search",
                    "--system",
                    "Sol",
                    "--planet-type",
                    "rocky body",
                    "--atmosphere-type",
                    "water",
                ],
                planet_type: "rocky body",
                atmosphere_type: "water",
                temperature: "0-5000",
                gravity: "0.1-1.0",
                pressure: "0.0-10.0",
                count: 5,
                exclude_with_bio_signals: false,
            },
            Case {
                name: "short flags with explicit ranges",
                arguments: vec![
                    "ed_planet_search",
                    "-S",
                    "Sol",
                    "-P",
                    "any",
                    "-A",
                    "any",
                    "-t",
                    "100-900",
                    "-g",
                    "0.2-0.8",
                    "-p",
                    "0.0-2.5",
                    "-c",
                    "12",
                ],
                planet_type: "any",
                atmosphere_type: "any",
                temperature: "100-900",
                gravity: "0.2-0.8",
                pressure: "0.0-2.5",
                count: 12,
                exclude_with_bio_signals: false,
            },
            Case {
                name: "bio-signal exclusion flag",
                arguments: vec![
                    "ed_planet_search",
                    "-S",
                    "Sol",
                    "-P",
                    "any",
                    "-A",
                    "any",
                    "--exclude-with-bio-signals",
                ],
                planet_type: "any",
                atmosphere_type: "any",
                temperature: "0-5000",
                gravity: "0.1-1.0",
                pressure: "0.0-10.0",
                count: 5,
                exclude_with_bio_signals: true,
            },
        ];

        for case in cases {
            let args = Args::try_parse_from(case.arguments)
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            assert_eq!(args.system, "Sol", "{}", case.name);
            assert_eq!(args.planet_type, case.planet_type, "{}", case.name);
            assert_eq!(args.atmosphere_type, case.atmosphere_type, "{}", case.name);
            assert_eq!(args.temperature, case.temperature, "{}", case.name);
            assert_eq!(args.gravity, case.gravity, "{}", case.name);
            assert_eq!(args.pressure, case.pressure, "{}", case.name);
            assert_eq!(args.count, case.count, "{}", case.name);
            assert_eq!(
                args.exclude_with_bio_signals, case.exclude_with_bio_signals,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn converts_cli_arguments_to_typed_search_criteria() {
        struct Case {
            name: &'static str,
            arguments: Vec<&'static str>,
            surface: Filter<SurfaceType>,
            atmosphere: Filter<AtmosphereType>,
            temperature: (f64, f64),
            gravity: (f64, f64),
            pressure: (f64, f64),
            count: usize,
            exclude_with_bio_signals: bool,
        }

        let cases = [
            Case {
                name: "any filters use unfiltered selections and defaults",
                arguments: vec![
                    "ed_planet_search",
                    "--system",
                    "Sol",
                    "--planet-type",
                    "any",
                    "--atmosphere-type",
                    "any",
                ],
                surface: Filter::Any,
                atmosphere: Filter::Any,
                temperature: (0.0, 5_000.0),
                gravity: (0.1, 1.0),
                pressure: (0.0, 10.0),
                count: 5,
                exclude_with_bio_signals: false,
            },
            Case {
                name: "canonical values and explicit ranges",
                arguments: vec![
                    "ed_planet_search",
                    "--system",
                    "  Sol  ",
                    "--planet-type",
                    "Rocky body",
                    "--atmosphere-type",
                    "Water",
                    "--temperature",
                    "395-450",
                    "--gravity",
                    "0.1-1.0",
                    "--pressure",
                    "0.040-0.063",
                    "--count",
                    "6",
                    "--exclude-with-bio-signals",
                ],
                surface: Filter::one_of(vec![SurfaceType::RockyBody]),
                atmosphere: Filter::one_of(vec![AtmosphereType::Water]),
                temperature: (395.0, 450.0),
                gravity: (0.1, 1.0),
                pressure: (0.040, 0.063),
                count: 6,
                exclude_with_bio_signals: true,
            },
            Case {
                name: "legacy aliases normalize to canonical enum values",
                arguments: vec![
                    "ed_planet_search",
                    "--system",
                    "Sol",
                    "--planet-type",
                    "rocky",
                    "--atmosphere-type",
                    "carbon-dioxide",
                ],
                surface: Filter::one_of(vec![SurfaceType::RockyBody]),
                atmosphere: Filter::one_of(vec![AtmosphereType::CarbonDioxide]),
                temperature: (0.0, 5_000.0),
                gravity: (0.1, 1.0),
                pressure: (0.0, 10.0),
                count: 5,
                exclude_with_bio_signals: false,
            },
        ];

        for case in cases {
            let config = Args::try_parse_from(case.arguments)
                .unwrap_or_else(|error| panic!("{}: {error}", case.name))
                .into_search_config()
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));

            assert_eq!(config.system, "Sol", "{}", case.name);
            assert_eq!(config.criteria.surface, case.surface, "{}", case.name);
            assert_eq!(config.criteria.atmosphere, case.atmosphere, "{}", case.name);
            assert_eq!(
                (config.criteria.temperature.min, config.criteria.temperature.max),
                case.temperature,
                "{}",
                case.name
            );
            assert_eq!(
                (config.criteria.gravity.min, config.criteria.gravity.max),
                case.gravity,
                "{}",
                case.name
            );
            assert_eq!(
                (config.criteria.pressure.min, config.criteria.pressure.max),
                case.pressure,
                "{}",
                case.name
            );
            assert_eq!(config.criteria.count, case.count, "{}", case.name);
            assert_eq!(
                config.criteria.exclude_with_bio_signals,
                case.exclude_with_bio_signals,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn rejects_reversed_ranges_while_building_configuration() {
        let error = Args::try_parse_from([
            "ed_planet_search",
            "--system",
            "Sol",
            "--planet-type",
            "any",
            "--atmosphere-type",
            "any",
            "--temperature",
            "500-100",
        ])
        .unwrap()
        .into_search_config()
        .unwrap_err();

        assert!(error.to_string().contains("температуры"));
    }
}
