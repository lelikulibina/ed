use std::io::{self, Write};

use anyhow::{Context, Result, anyhow};

use crate::{
    application::presets::{PRESETS, SearchPreset},
    cli::{parse_range_f32, parse_range_i32},
    domain::criteria::{NumericRange, SearchCriteria},
};

pub struct TuiSelection {
    pub system: String,
    pub criteria: SearchCriteria,
}

pub fn run_tui() -> Result<Option<TuiSelection>> {
    loop {
        draw_menu(0);
        println!();
        let choice = prompt("Выберите профиль [1–9]")?;
        let choice = match parse_menu_choice(&choice) {
            Ok(choice) => choice,
            Err(_) => {
                println!("Введите число от 1 до 9.");
                pause()?;
                continue;
            }
        };
        match choice {
            9 => return Ok(None),
            8 => return run_manual_setup().map(Some),
            number @ 1..=7 => {
                let preset = &PRESETS[usize::from(number - 1)];
                draw_preset(preset);
                return configure_search(preset.criteria(5, false)?).map(Some);
            }
            _ => {
                println!("Введите число от 1 до 9.");
                pause()?;
            }
        }
    }
}

fn draw_menu(_selected: usize) {
    print!("\x1B[2J\x1B[H");
    println!("\x1B[1;36m╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                         ED: PLANET SEARCH                            ║");
    println!("║                 ПОДБОР ПЛАНЕТ ДЛЯ ЭКЗОБИОЛОГИИ                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\x1B[0m");
    println!();
    println!("  \x1B[1;37mПОИСКОВЫЕ ПРОФИЛИ\x1B[0m");
    println!("  ────────────────────────────────────────────────────────────────────");
    for preset in PRESETS {
        println!("  \x1B[36m{}\x1B[0m  {}", preset.number, preset.name);
    }
    println!("  \x1B[36m8\x1B[0m  Ручная настройка");
    println!("  \x1B[90m9  Выход\x1B[0m");
    println!();
    println!("  Справочные значения видов и стоимости не являются гарантией находки.");
    println!("  Вулканизм не фильтруется: нет надёжного источника для этих критериев.");
    let _ = io::stdout().flush();
}

fn draw_preset(preset: &SearchPreset) {
    print!("\x1B[2J\x1B[H");
    println!("\x1B[1;36m╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                         ED: PLANET SEARCH                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\x1B[0m");
    println!();
    println!(
        "  \x1B[1;37mПРОФИЛЬ {:02}: {}\x1B[0m",
        preset.number, preset.name
    );
    println!("  Тип поверхности : {}", preset.surface_types.join(" ИЛИ "));
    println!("  Атмосфера        : {}", preset.atmosphere);
    println!(
        "  Температура      : {:.0}–{:.0} K",
        preset.temperature.min, preset.temperature.max
    );
    println!(
        "  Давление         : {:.3}–{:.3} atm",
        preset.pressure.min, preset.pressure.max
    );
    println!("  Гравитация       : 0.1–1.0 g (стандартный диапазон)");
    println!("  Вулканизм        : не фильтруется");
    println!();
    println!(
        "  \x1B[90mСправочно: до {} дорогих видов, потенциал {:>3} млн CR\x1B[0m",
        preset.expensive_species,
        preset.potential_value_cr / 1_000_000
    );
    println!();
}

fn configure_search(criteria: SearchCriteria) -> Result<TuiSelection> {
    let system = prompt("Центральная система (например, Sol)")?;
    if system.trim().is_empty() {
        return Err(anyhow!("Название центральной системы не может быть пустым"));
    }
    let count = prompt("Количество результатов [5]")?;
    let count = if count.trim().is_empty() {
        5
    } else {
        count
            .trim()
            .parse::<usize>()
            .context("Количество результатов должно быть целым числом")?
    };
    if count == 0 {
        return Err(anyhow!("Количество результатов должно быть больше нуля"));
    }
    let exclude = prompt("Исключать тела с зарегистрированными биосигналами? [н]")?;
    let exclude = matches!(
        exclude.trim().to_ascii_lowercase().as_str(),
        "д" | "да" | "y" | "yes"
    );
    Ok(TuiSelection {
        system: system.trim().to_owned(),
        criteria: criteria
            .with_bio_signal_exclusion(exclude)
            .with_count(count)?,
    })
}

fn run_manual_setup() -> Result<TuiSelection> {
    println!("\n\x1B[1;36mРУЧНАЯ НАСТРОЙКА\x1B[0m");
    let system = prompt("Центральная система")?;
    if system.trim().is_empty() {
        return Err(anyhow!("Название центральной системы не может быть пустым"));
    }
    let planet = prompt("Тип поверхности [any]")?;
    let atmosphere = prompt("Атмосфера [any]")?;
    let temperature = prompt("Температура K [0-5000]")?;
    let gravity = prompt("Гравитация g [0.1-1.0]")?;
    let pressure = prompt("Давление atm [0-10]")?;
    let count = prompt("Количество результатов [5]")?;
    let temperature = parse_range_i32(if temperature.trim().is_empty() {
        "0-5000"
    } else {
        temperature.trim()
    })?;
    let gravity = parse_range_f32(if gravity.trim().is_empty() {
        "0.1-1.0"
    } else {
        gravity.trim()
    })?;
    let pressure = parse_range_f32(if pressure.trim().is_empty() {
        "0-10"
    } else {
        pressure.trim()
    })?;
    let count = if count.trim().is_empty() {
        5
    } else {
        count.trim().parse::<usize>()?
    };
    let exclude = prompt("Исключать тела с зарегистрированными биосигналами? [н]")?;
    let exclude = matches!(
        exclude.trim().to_ascii_lowercase().as_str(),
        "д" | "да" | "y" | "yes"
    );
    let criteria = SearchCriteria::from_cli(
        if planet.trim().is_empty() {
            "any"
        } else {
            planet.trim()
        },
        if atmosphere.trim().is_empty() {
            "any"
        } else {
            atmosphere.trim()
        },
        NumericRange::new(f64::from(temperature.0), f64::from(temperature.1))
            .ok_or_else(|| anyhow!("Некорректная температура"))?,
        NumericRange::new(f64::from(gravity.0), f64::from(gravity.1))
            .ok_or_else(|| anyhow!("Некорректная гравитация"))?,
        NumericRange::new(f64::from(pressure.0), f64::from(pressure.1))
            .ok_or_else(|| anyhow!("Некорректное давление"))?,
        count,
    )?
    .with_bio_signal_exclusion(exclude);
    Ok(TuiSelection {
        system: system.trim().to_owned(),
        criteria,
    })
}

fn prompt(label: &str) -> Result<String> {
    print!("  {label}: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim_end().to_owned())
}

fn pause() -> Result<()> {
    let _ = prompt("Нажмите Enter для продолжения")?;
    Ok(())
}

fn parse_menu_choice(value: &str) -> Result<u8> {
    value
        .trim()
        .parse::<u8>()
        .map_err(|_| anyhow!("Введите число от 1 до 9"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_choice_parser_accepts_numbers_and_rejects_text() {
        for choice in 1..=9 {
            assert_eq!(parse_menu_choice(&choice.to_string()).unwrap(), choice);
        }
        assert!(parse_menu_choice("0").is_ok());
        assert!(parse_menu_choice("q").is_err());
    }
}
