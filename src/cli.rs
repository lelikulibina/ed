use anyhow::{Context, Result, anyhow};
use clap::Parser;

use crate::{
    application::config::SearchConfig,
    domain::criteria::{NumericRange, SearchCriteria},
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
        let (temperature_min, temperature_max) = parse_range_i32(&self.temperature)
            .context("Температура: ожидается диапазон min-max в K")?;
        let (gravity_min, gravity_max) =
            parse_range_f64(&self.gravity).context("Гравитация: ожидается диапазон min-max в g")?;
        let (pressure_min, pressure_max) = parse_range_f64(&self.pressure)
            .context("Давление: ожидается диапазон min-max в atm")?;

        let temperature = NumericRange::new(f64::from(temperature_min), f64::from(temperature_max))
            .ok_or_else(|| anyhow!("Некорректный диапазон температуры: {}", self.temperature))?;
        let gravity = NumericRange::new(gravity_min, gravity_max)
            .ok_or_else(|| anyhow!("Некорректный диапазон гравитации: {}", self.gravity))?;
        let pressure = NumericRange::new(pressure_min, pressure_max)
            .ok_or_else(|| anyhow!("Некорректный диапазон давления: {}", self.pressure))?;

        let criteria = SearchCriteria::from_cli(
            &self.planet_type,
            &self.atmosphere_type,
            temperature,
            gravity,
            pressure,
            self.count,
        )?
        .with_bio_signal_exclusion(self.exclude_with_bio_signals);

        SearchConfig::new(self.system, criteria).map_err(Into::into)
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
    use clap::Parser;

    #[test]
    fn keeps_legacy_long_flags_and_defaults() {
        let args = Args::try_parse_from([
            "ed_planet_search",
            "--system",
            "Sol",
            "--planet-type",
            "rocky body",
            "--atmosphere-type",
            "water",
        ])
        .unwrap();
        assert_eq!(args.system, "Sol");
        assert_eq!(args.planet_type, "rocky body");
        assert_eq!(args.atmosphere_type, "water");
        assert_eq!(args.temperature, "0-5000");
        assert_eq!(args.gravity, "0.1-1.0");
        assert_eq!(args.pressure, "0.0-10.0");
        assert_eq!(args.count, 5);
        assert!(!args.exclude_with_bio_signals);
    }

    #[test]
    fn parses_exclude_with_bio_signals_flag() {
        let args = Args::try_parse_from([
            "ed_planet_search",
            "-S",
            "Sol",
            "-P",
            "any",
            "-A",
            "any",
            "--exclude-with-bio-signals",
        ])
        .unwrap();
        assert!(args.exclude_with_bio_signals);
    }

    #[test]
    fn keeps_legacy_short_flags_and_range_values() {
        let args = Args::try_parse_from([
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
        ])
        .unwrap();
        assert_eq!(args.system, "Sol");
        assert_eq!(args.temperature, "100-900");
        assert_eq!(args.gravity, "0.2-0.8");
        assert_eq!(args.pressure, "0.0-2.5");
        assert_eq!(args.count, 12);
    }

    #[test]
    fn converts_arguments_to_shared_search_configuration() {
        let config = Args::try_parse_from([
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
        ])
        .unwrap()
        .into_search_config()
        .unwrap();

        assert_eq!(config.system, "Sol");
        assert_eq!(config.criteria.count, 6);
        assert!(config.criteria.exclude_with_bio_signals);
        assert_eq!(config.criteria.temperature.min, 395.0);
        assert_eq!(config.criteria.temperature.max, 450.0);
        assert_eq!(config.criteria.gravity.min, 0.1);
        assert_eq!(config.criteria.gravity.max, 1.0);
        assert_eq!(config.criteria.pressure.min, 0.040);
        assert_eq!(config.criteria.pressure.max, 0.063);
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
