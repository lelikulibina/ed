use std::collections::BTreeMap;

use crate::domain::{
    body::Body,
    exobiology::{self, CandidateStatus, EstimateError},
};

pub(crate) fn exobiology_summary(body: &Body) -> Result<Option<String>, EstimateError> {
    let estimate = exobiology::estimate(body)?;

    if estimate.biological_signal_count == Some(0) {
        return Ok(Some(
            "Биологических сигналов Spansh не обнаружил.".to_owned(),
        ));
    }

    if estimate.genera.is_empty() {
        if let Some(genera) = estimate
            .biological_genera
            .as_ref()
            .filter(|genera| !genera.is_empty())
        {
            return Ok(Some(format!(
                "Spansh указал биологические группы {}, но подходящие виды в справочнике не найдены.",
                genera
                    .iter()
                    .map(|genus| russian_group_name(genus))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        return Ok(None);
    }

    let mut lines = Vec::new();
    let confirmed_genera = estimate
        .biological_genera
        .as_ref()
        .is_some_and(|genera| !genera.is_empty());
    lines.push(if confirmed_genera {
        "Возможные виды из биологических групп, указанных Spansh:".to_owned()
    } else {
        "Возможные виды по параметрам планеты (биологические группы не уточнены):".to_owned()
    });

    let rows = estimate
        .genera
        .iter()
        .flat_map(|genus| {
            genus.candidates.iter().map(move |candidate| {
                let status = match candidate.status {
                    CandidateStatus::EligibleByKnownData => {
                        "По известным условиям подходит".to_owned()
                    }
                    CandidateStatus::NeedsVerification => {
                        format!("Проверить: {}", candidate.unknown_conditions.join(", "))
                    }
                };
                vec![
                    russian_group_name(&genus.genus).to_owned(),
                    candidate.species.clone(),
                    format!("{} CR", format_credits(candidate.base_value_cr)),
                    status,
                ]
            })
        })
        .collect::<Vec<_>>();

    lines.extend(render_table(
        &["Биологическая группа", "Вид", "За 3 образца", "Статус"],
        &rows,
    ));

    lines.push(format!(
        "Потенциальная базовая выручка: 0–{} CR",
        format_credits(estimate.maximum_base_value_cr)
    ));
    lines.push(format!(
        "При First Logged ×5: 0–{} CR (условный сценарий)",
        format_credits(estimate.maximum_first_logged_value_cr)
    ));

    let mut notes = Vec::new();
    if !estimate.catalog_is_complete {
        notes.push("справочник видов неполный");
    }
    notes.push("вероятности появления видов не рассчитаны");
    notes.push("учтены только условия, внесённые в справочник");
    notes
        .push("внутри одной биологической группы учитывается один вид, а не сумма всех кандидатов");
    lines.push(format!("Примечание: {}.", notes.join("; ")));

    Ok(Some(lines.join("\n")))
}

/// Builds a compact, dynamically sized text table suitable for a terminal.
fn render_table(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let widths = headers
        .iter()
        .enumerate()
        .map(|(column, header)| {
            rows.iter()
                .map(|row| row.get(column).map_or(0, |cell| cell.chars().count()))
                .fold(header.chars().count(), usize::max)
        })
        .collect::<Vec<_>>();

    let border = |left: char, middle: char, right: char| {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|width| "─".repeat(width + 2))
                .collect::<Vec<_>>()
                .join(&middle.to_string())
        )
    };
    let format_row = |cells: &[String]| {
        format!(
            "│ {} │",
            widths
                .iter()
                .enumerate()
                .map(|(index, width)| {
                    let value = cells.get(index).map_or("", String::as_str);
                    format!("{value:<width$}")
                })
                .collect::<Vec<_>>()
                .join(" │ ")
        )
    };

    let mut output = vec![border('┌', '┬', '┐')];
    output.push(format_row(
        &headers
            .iter()
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>(),
    ));
    output.push(border('├', '┼', '┤'));
    output.extend(rows.iter().map(|row| format_row(row)));
    output.push(border('└', '┴', '┘'));
    output
}

pub(crate) fn russian_group_name(name: &str) -> &str {
    match name {
        "Aleoida" => "Алеоиды",
        "Bacterium" => "Бактерии",
        "Cactoida" => "Кактусоиды",
        "Clypeus" => "Клипеусы",
        "Concha" => "Моллюски",
        "Fonticulua" => "Фонтикулы",
        "Frutexa" => "Кустарники",
        "Fumerola" => "Дымники",
        "Fungoida" => "Фунгоиды",
        "Osseus" => "Оссии",
        "Recepta" => "Рецептоиды",
        "Stratum" => "Стратум",
        "Tubus" => "Тубусы",
        "Tussock" => "Дерновина",
        other => other,
    }
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
            last_updated_at: None,
            surface_temperature: None,
            surface_pressure: None,
            gravity: None,
            atmosphere: None,
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn species_candidates_are_rendered_as_a_structured_table() {
        let mut body = body("Test body", "100", "Test system");
        body.subtype = Some("Rocky body".to_owned());
        body.atmosphere = Some("Water".to_owned());
        body.surface_temperature = Some(200.0);
        body.gravity = Some(0.2);

        let summary = exobiology_summary(&body).unwrap().unwrap();
        assert!(summary.contains("┌"));
        assert!(summary.contains("Биологическая группа"));
        assert!(summary.contains("Оссии"));
        assert!(!summary.contains("First Footfall"));
        assert!(summary.contains("Вид"));
        assert!(summary.contains("За 3 образца"));
        assert!(summary.contains("Потенциальная базовая выручка: 0–"));
        assert!(summary.contains("При First Logged ×5: 0–"));
        assert!(summary.contains("Проверить: тип вулканизма"));
        assert!(!summary.contains("Вероятности появления видов не рассчитаны.\n"));
    }

    #[test]
    fn confirmed_spansh_genera_limit_the_species_table() {
        let mut body = body("Test body", "100", "Test system");
        body.subtype = Some("Rocky body".to_owned());
        body.atmosphere = Some("Water".to_owned());
        body.surface_temperature = Some(200.0);
        body.gravity = Some(0.2);
        body.biological_signal_count = Some(2);
        body.geological_signal_count = Some(1);
        body.biological_genera = Some(vec!["Osseus".to_owned(), "Tussock".to_owned()]);

        let summary = exobiology_summary(&body).unwrap().unwrap();
        assert!(summary.contains("из биологических групп, указанных Spansh"));
        assert!(summary.contains("Оссии"));
        assert!(summary.contains("Дерновина"));
        assert!(summary.contains("справочник видов неполный"));
        assert!(!summary.contains("Бактерии"));
    }

    #[test]
    fn zero_biological_signals_are_reported_without_species_candidates() {
        let mut body = body("Test body", "100", "Test system");
        body.biological_signal_count = Some(0);

        let summary = exobiology_summary(&body).unwrap().unwrap();
        assert!(summary.contains("Биологических сигналов Spansh не обнаружил"));
        assert!(!summary.contains("Вид-кандидат"));
    }

    #[test]
    fn no_matching_species_returns_none_when_no_genera_are_known() {
        let mut body = body("Test body", "100", "Test system");
        body.subtype = Some("Icy body".to_owned());
        body.atmosphere = Some("Neon".to_owned());
        body.surface_temperature = Some(40.0);
        body.gravity = Some(1.0);
        body.volcanism_type = Some("No volcanism".to_owned());
        body.geological_signal_count = Some(0);

        assert_eq!(exobiology_summary(&body).unwrap(), None);
    }

    #[test]
    fn russian_group_names_are_localized() {
        assert_eq!(russian_group_name("Osseus"), "Оссии");
        assert_eq!(russian_group_name("Tussock"), "Дерновина");
        assert_eq!(russian_group_name("Bacterium"), "Бактерии");
    }

    #[test]
    fn table_columns_expand_to_fit_the_longest_cell() {
        let lines = render_table(
            &["Биологическая группа", "Вид"],
            &[vec!["Osseus".to_owned(), "Discus".to_owned()]],
        );
        assert!(lines[0].contains("┌"));
        assert!(lines[3].contains("Osseus"));
        assert!(lines[3].contains("Discus"));
        assert!(lines.last().unwrap().contains("└"));
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
