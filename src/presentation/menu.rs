use std::io::{self, BufRead, Write};

use crate::{
    application::profiles::{SEARCH_PROFILES, SearchProfile},
    cli::{parse_range_f32, parse_range_i32},
    domain::criteria::{NumericRange, SearchCriteria},
};

pub enum MenuAction {
    Exit,
    Search {
        system: String,
        criteria: SearchCriteria,
    },
}

pub fn run_menu<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<MenuAction> {
    loop {
        writeln!(writer, "\n=== ED: Planet Search ===")?;
        for profile in SEARCH_PROFILES {
            writeln!(writer, "{}. {}", profile.number, profile.name)?;
        }
        writeln!(writer, "8. Ручная настройка")?;
        writeln!(writer, "9. Выход")?;
        write!(writer, "Выберите профиль: ")?;
        writer.flush()?;
        let Some(choice) = read_line(reader)? else {
            return Ok(MenuAction::Exit);
        };
        match choice.trim().parse::<usize>() {
            Ok(9) => return Ok(MenuAction::Exit),
            Ok(n @ 1..=7) => {
                let profile = &SEARCH_PROFILES[n - 1];
                display_profile(writer, profile)?;
                let Some(system) = prompt_nonempty(reader, writer, "Центральная система: ")?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(count) = prompt_count(reader, writer)? else {
                    return Ok(MenuAction::Exit);
                };
                return Ok(MenuAction::Search {
                    system,
                    criteria: profile.criteria(count).expect("static profile is valid"),
                });
            }
            Ok(8) => {
                let Some(system) = prompt_nonempty(reader, writer, "Центральная система: ")?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(planet) = prompt_allowed(
                    reader,
                    writer,
                    "Тип поверхности (rocky body / icy body / rocky ice world / high metal content world / any): ",
                    &[
                        "Rocky body",
                        "Icy body",
                        "Rocky ice world",
                        "High metal content world",
                        "any",
                    ],
                )?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(atmosphere) = prompt_allowed(
                    reader,
                    writer,
                    "Атмосфера (ammonia / water / carbon dioxide / sulphur dioxide / argon / oxygen / any): ",
                    &[
                        "Ammonia",
                        "Water",
                        "Carbon dioxide",
                        "Sulphur dioxide",
                        "Nitrogen",
                        "Methane",
                        "Helium",
                        "Silicate vapour",
                        "Neon",
                        "Argon",
                        "Oxygen",
                        "any",
                    ],
                )?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(temperature) =
                    prompt_range_i32(reader, writer, "Температура, K (min-max): ")?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(gravity) = prompt_range_f32(reader, writer, "Гравитация, g (min-max): ")?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(pressure) = prompt_range_f32(reader, writer, "Давление, atm (min-max): ")?
                else {
                    return Ok(MenuAction::Exit);
                };
                let Some(count) = prompt_count(reader, writer)? else {
                    return Ok(MenuAction::Exit);
                };
                let criteria = SearchCriteria::from_cli(
                    &planet,
                    &atmosphere,
                    temperature,
                    gravity,
                    pressure,
                    count,
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
                return Ok(MenuAction::Search { system, criteria });
            }
            _ => writeln!(writer, "Неверный пункт. Введите число от 1 до 9.")?,
        }
    }
}

fn display_profile<W: Write>(writer: &mut W, p: &SearchProfile) -> io::Result<()> {
    writeln!(writer, "\n{} — {}", p.number, p.name)?;
    writeln!(writer, "Тип мира: {}", p.planet_types.join(" / "))?;
    writeln!(writer, "Атмосфера: {}", p.atmosphere)?;
    writeln!(
        writer,
        "Температура: {}–{} K",
        p.temperature.0, p.temperature.1
    )?;
    writeln!(
        writer,
        "Давление: {:.3}–{:.3} atm",
        p.pressure.0, p.pressure.1
    )?;
    writeln!(
        writer,
        "Гравитация: {:.1}–{:.1} g",
        p.gravity.0, p.gravity.1
    )?;
    writeln!(writer, "Вулканизм: {}", p.volcanism.description())?;
    writeln!(
        writer,
        "Максимум дорогих видов: {}",
        p.max_expensive_species
    )?;
    writeln!(
        writer,
        "Потенциальный максимум: {} CR (теоретический ориентир, не гарантия)",
        p.potential_max_credits
    )
}

fn read_line<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        Ok(None)
    } else {
        Ok(Some(line.trim().to_owned()))
    }
}
fn prompt_nonempty<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    prompt: &str,
) -> io::Result<Option<String>> {
    loop {
        write!(w, "{prompt}")?;
        w.flush()?;
        let Some(value) = read_line(r)? else {
            return Ok(None);
        };
        if !value.is_empty() {
            return Ok(Some(value));
        }
        writeln!(w, "Значение не должно быть пустым.")?;
    }
}
fn prompt_count<R: BufRead, W: Write>(r: &mut R, w: &mut W) -> io::Result<Option<usize>> {
    loop {
        write!(w, "Количество результатов [5]: ")?;
        w.flush()?;
        let Some(value) = read_line(r)? else {
            return Ok(None);
        };
        if value.is_empty() {
            return Ok(Some(5));
        }
        if let Ok(n) = value.parse::<usize>()
            && n > 0
        {
            return Ok(Some(n));
        }
        writeln!(w, "Введите целое число больше нуля.")?;
    }
}
fn prompt_allowed<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    prompt: &str,
    allowed: &[&str],
) -> io::Result<Option<String>> {
    loop {
        let Some(value) = prompt_nonempty(r, w, prompt)? else {
            return Ok(None);
        };
        if let Some(canonical) = allowed
            .iter()
            .find(|item| item.eq_ignore_ascii_case(&value))
        {
            return Ok(Some((*canonical).to_owned()));
        }
        writeln!(
            w,
            "Недопустимое значение. Выберите один из перечисленных вариантов."
        )?;
    }
}
fn prompt_range_i32<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    prompt: &str,
) -> io::Result<Option<NumericRange>> {
    loop {
        write!(w, "{prompt}")?;
        w.flush()?;
        let Some(v) = read_line(r)? else {
            return Ok(None);
        };
        if let Ok((a, b)) = parse_range_i32(&v)
            && let Some(range) = NumericRange::new(f64::from(a), f64::from(b))
        {
            return Ok(Some(range));
        }
        writeln!(
            w,
            "Неверный диапазон. Используйте min-max, где min не больше max."
        )?;
    }
}
fn prompt_range_f32<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    prompt: &str,
) -> io::Result<Option<NumericRange>> {
    loop {
        write!(w, "{prompt}")?;
        w.flush()?;
        let Some(v) = read_line(r)? else {
            return Ok(None);
        };
        if let Ok((a, b)) = parse_range_f32(&v)
            && let Some(range) = NumericRange::new(f64::from(a), f64::from(b))
        {
            return Ok(Some(range));
        }
        writeln!(
            w,
            "Неверный диапазон. Используйте min-max, где min не больше max."
        )?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn eof_exits_menu_without_panic() {
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        assert!(matches!(
            run_menu(&mut input, &mut output).unwrap(),
            MenuAction::Exit
        ));
    }
    #[test]
    fn invalid_choice_reprompts_and_exit_is_clean() {
        let mut input = Cursor::new(b"0\n9\n".to_vec());
        let mut output = Vec::new();
        assert!(matches!(
            run_menu(&mut input, &mut output).unwrap(),
            MenuAction::Exit
        ));
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Неверный пункт")
        );
    }
    #[test]
    fn profile_accepts_blank_count_as_five() {
        let mut input = Cursor::new(b"4\nSol\n\n".to_vec());
        let mut output = Vec::new();
        match run_menu(&mut input, &mut output).unwrap() {
            MenuAction::Search { criteria, system } => {
                assert_eq!(system, "Sol");
                assert_eq!(criteria.count, 5);
            }
            MenuAction::Exit => panic!("expected search"),
        }
    }
    #[test]
    fn eof_during_profile_input_exits_cleanly() {
        let mut input = Cursor::new(b"4\\nSol\\n".to_vec());
        let mut output = Vec::new();
        assert!(matches!(
            run_menu(&mut input, &mut output).unwrap(),
            MenuAction::Exit
        ));
    }

    #[test]
    fn manual_mode_reprompts_after_invalid_surface_type() {
        let mut input = Cursor::new(
            b"8\nSol\nnot-a-type\nrocky body\nwater\n100-200\n0.1-1.0\n0.1-1.0\n\n".to_vec(),
        );
        let mut output = Vec::new();
        match run_menu(&mut input, &mut output).unwrap() {
            MenuAction::Search { criteria, .. } => assert_eq!(criteria.planet_type, "Rocky body"),
            MenuAction::Exit => panic!("expected manual search"),
        }
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Недопустимое значение")
        );
    }

    #[test]
    fn profile_five_accepts_verified_water_magma_and_geyser_values() {
        let mut input = Cursor::new(b"5\nSol\n\n".to_vec());
        let mut output = Vec::new();
        match run_menu(&mut input, &mut output).unwrap() {
            MenuAction::Search { criteria, system } => {
                assert_eq!(system, "Sol");
                assert_eq!(criteria.count, 5);
                assert_eq!(criteria.volcanism, crate::domain::criteria::VolcanismFilter::WaterMagmaAndGeysers);
            }
            MenuAction::Exit => panic!("expected profile search"),
        }
    }
}
