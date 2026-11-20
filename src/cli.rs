use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "ED:Planet Search",
    version,
    about = "Поисковик планет в базе EDSM (с расширеной фильтрацией).\n\
        Позволяет искать планеты по типу поверхности, атмосфере и физическим параметрам"
)]
pub(crate) struct Args {
    #[arg(
        short = 'S',
        long,
        help = "Название сектора в котором будет производится поиск. Например: \"Col 285 Sector BQ-Y d40\""
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
        help = "Диапазон температуры поверхности планеты (int). Формат: min-max"
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
}

pub(crate) fn parse_range_f32(s: &str) -> Result<(f32, f32)> {
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
            "ed_planet_search", "--system", "Sol",
            "--planet-type", "rocky body", "--atmosphere-type", "water",
        ]).unwrap();
        assert_eq!(args.system, "Sol");
        assert_eq!(args.planet_type, "rocky body");
        assert_eq!(args.atmosphere_type, "water");
        assert_eq!(args.temperature, "0-5000");
        assert_eq!(args.gravity, "0.1-1.0");
        assert_eq!(args.pressure, "0.0-10.0");
        assert_eq!(args.count, 5);
    }

    #[test]
    fn keeps_legacy_short_flags_and_range_values() {
        let args = Args::try_parse_from([
            "ed_planet_search", "-S", "Sol", "-P", "any", "-A", "any",
            "-t", "100-900", "-g", "0.2-0.8", "-p", "0.0-2.5", "-c", "12",
        ]).unwrap();
        assert_eq!(args.system, "Sol");
        assert_eq!(args.temperature, "100-900");
        assert_eq!(args.gravity, "0.2-0.8");
        assert_eq!(args.pressure, "0.0-2.5");
        assert_eq!(args.count, 12);
    }
}
