use std::collections::BTreeMap;

use crate::domain::{
    body::Body,
    exobiology::{self, CandidateStatus, EstimateError},
};

pub fn print_results(bodies: &[Body]) {
    if bodies.is_empty() {
        println!("❌ Не найдено подходящих планет.");
        return;
    }

    for (system_id, planets) in group_by_system(bodies) {
        let system_name = planets
            .iter()
            .find_map(|body| body.system_name.as_deref())
            .unwrap_or("неизвестная система");
        println!("Система: {system_name} (ID64: {system_id})");
        for body in planets {
            println!(
                "  • {}{}\n    Тип: {}\n    Атмосфера: {}\n    Температура: {} K\n    Давление: {}\n    Гравитация: {} g\n    Расстояние от центра поиска: {} св. л.\n",
                body.name,
                if body.is_mapped == Some(true) {
                    " [изучена]"
                } else {
                    ""
                },
                body.subtype.as_deref().unwrap_or("неизвестно"),
                body.atmosphere.as_deref().unwrap_or("неизвестно"),
                display_number(body.surface_temperature),
                display_number(body.surface_pressure),
                display_number(body.gravity),
                display_number(body.distance_ly),
            );
            match exobiology_summary(body) {
                Ok(Some(summary)) => println!("{summary}"),
                Ok(None) => {}
                Err(error) => println!("    Не удалось рассчитать экзобиологию: {error}"),
            }
        }
    }
}

fn exobiology_summary(body: &Body) -> Result<Option<String>, EstimateError> {
    let estimate = exobiology::estimate(body)?;
    if estimate.genera.is_empty() {
        return Ok(None);
    }

    let mut lines = vec!["    Экзобиология — предварительный прогноз:".to_owned()];
    for genus in &estimate.genera {
        let candidates = genus
            .candidates
            .iter()
            .map(|candidate| {
                let verification = match candidate.status {
                    CandidateStatus::EligibleByKnownData => String::new(),
                    CandidateStatus::NeedsVerification => format!(
                        " [нужна проверка: {}]",
                        candidate.unknown_conditions.join(", ")
                    ),
                };
                format!(
                    "{} — {} CR{}",
                    candidate.species,
                    format_credits(candidate.base_value_cr),
                    verification
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(format!("      {}: {}", genus.genus, candidates));
    }

    lines.push(format!(
        "      Потенциальная стоимость (базовая): 0–{} CR",
        format_credits(estimate.maximum_base_value_cr)
    ));
    lines.push(format!(
        "      Если доступен First Logged: 0–{} CR",
        format_credits(estimate.maximum_first_logged_value_cr)
    ));
    lines.push(
        "      Биосигналы не учтены; верхняя граница предполагает наличие всех указанных родов."
            .to_owned(),
    );
    lines.push("      Вероятности появления видов не рассчитаны.".to_owned());

    Ok(Some(lines.join("\n")))
}

fn format_credits(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(' ');
        }
        formatted.push(character);
    }
    formatted
}

fn group_by_system(bodies: &[Body]) -> BTreeMap<String, Vec<&Body>> {
    let mut systems: BTreeMap<String, Vec<&Body>> = BTreeMap::new();
    for body in bodies {
        let system_id = body
            .system_id64
            .clone()
            .or_else(|| body.system_name.clone())
            .unwrap_or_else(|| "unknown".to_owned());
        systems.entry(system_id).or_default().push(body);
    }
    systems
}

fn display_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "нет данных".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(name: &str, system_id64: &str, system_name: &str) -> Body {
        Body {
            id64: None,
            name: name.to_owned(),
            system_id64: Some(system_id64.to_owned()),
            system_name: Some(system_name.to_owned()),
            subtype: None,
            distance_ly: None,
            is_landable: None,
            is_mapped: None,
            surface_temperature: None,
            surface_pressure: None,
            gravity: None,
            atmosphere: None,
        }
    }

    #[test]
    fn exobiology_summary_reports_range_and_separates_first_logged_scenario() {
        let mut body = body("Test body", "100", "Test system");
        body.subtype = Some("Rocky body".to_owned());
        body.atmosphere = Some("Water".to_owned());
        body.surface_temperature = Some(200.0);
        body.gravity = Some(0.2);

        let summary = exobiology_summary(&body).unwrap().unwrap();
        assert!(summary.contains("Экзобиология — предварительный прогноз"));
        assert!(summary.contains("Потенциальная стоимость (базовая): 0–"));
        assert!(summary.contains("Если доступен First Logged: 0–"));
        assert!(summary.contains("Вероятности появления видов не рассчитаны"));
        assert!(summary.contains("нужна проверка: тип вулканизма"));
    }

    #[test]
    fn exobiology_summary_is_omitted_when_no_species_match_known_conditions() {
        let mut body = body("Test body", "100", "Test system");
        body.subtype = Some("Icy body".to_owned());
        body.atmosphere = Some("Neon".to_owned());
        body.surface_temperature = Some(40.0);
        body.gravity = Some(1.0);

        assert_eq!(exobiology_summary(&body).unwrap(), None);
    }

    #[test]
    fn groups_by_system_id64_not_system_name() {
        let bodies = [
            body("A 1", "100", "System A"),
            body("A 2", "100", "Different spelling"),
            body("B 1", "200", "System A"),
        ];
        let groups = group_by_system(&bodies);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups["100"].len(), 2);
        assert_eq!(groups["200"].len(), 1);
    }
}
