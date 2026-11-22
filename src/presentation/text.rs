use crate::domain::{
    body::Body,
    exobiology::{self, CandidateStatus, EstimateError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExobiologyRow {
    pub genus: String,
    pub species: String,
    pub base_value_cr: u64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExobiologyView {
    pub headline: String,
    pub rows: Vec<ExobiologyRow>,
    pub summary: Vec<String>,
    pub notes: Vec<String>,
    pub maximum_base_value_cr: u64,
}

pub fn exobiology_view(body: &Body) -> Result<Option<ExobiologyView>, EstimateError> {
    let estimate = exobiology::estimate(body)?;

    if estimate.biological_signal_count == Some(0) {
        return Ok(Some(ExobiologyView {
            headline: "Биологических сигналов Spansh не обнаружил.".to_owned(),
            rows: Vec::new(),
            summary: Vec::new(),
            notes: Vec::new(),
            maximum_base_value_cr: 0,
        }));
    }

    if estimate.genera.is_empty() {
        if let Some(genera) = estimate
            .biological_genera
            .as_ref()
            .filter(|genera| !genera.is_empty())
        {
            return Ok(Some(ExobiologyView {
                headline: format!(
                    "Spansh указал биологические группы {}, но подходящие виды в справочнике не найдены.",
                    genera
                        .iter()
                        .map(|genus| russian_group_name(genus))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                rows: Vec::new(),
                summary: Vec::new(),
                notes: Vec::new(),
                maximum_base_value_cr: 0,
            }));
        }
        return Ok(None);
    }

    let confirmed_genera = estimate
        .biological_genera
        .as_ref()
        .is_some_and(|genera| !genera.is_empty());
    let headline = if confirmed_genera {
        "Возможные виды из биологических групп, указанных Spansh:"
    } else {
        "Возможные виды по параметрам планеты (биологические группы не уточнены):"
    }
    .to_owned();

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
                ExobiologyRow {
                    genus: russian_group_name(&genus.genus).to_owned(),
                    species: candidate.species.clone(),
                    base_value_cr: candidate.base_value_cr,
                    status,
                }
            })
        })
        .collect::<Vec<_>>();

    let summary = vec![
        format!(
            "Потенциальная базовая выручка: 0–{} CR",
            format_credits(estimate.maximum_base_value_cr)
        ),
        format!(
            "При First Logged ×5: 0–{} CR (условный сценарий)",
            format_credits(estimate.maximum_first_logged_value_cr)
        ),
    ];

    let mut notes = Vec::new();
    if !estimate.catalog_is_complete {
        notes.push("справочник видов неполный");
    }
    if !estimate.probabilities_calculated {
        notes.push("вероятности появления видов не рассчитаны");
    }
    notes.push("учтены только условия, внесённые в справочник");
    notes
        .push("внутри одной биологической группы учитывается один вид, а не сумма всех кандидатов");

    Ok(Some(ExobiologyView {
        headline,
        rows,
        summary,
        notes: vec![format!("Примечание: {}.", notes.join("; "))],
        maximum_base_value_cr: estimate.maximum_base_value_cr,
    }))
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

pub(crate) fn format_credits(value: u64) -> String {
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

    fn body() -> Body {
        Body {
            id64: Some("100".to_owned()),
            name: "Test body".to_owned(),
            system_id64: Some("200".to_owned()),
            system_name: Some("Test system".to_owned()),
            subtype: Some("Rocky body".to_owned()),
            distance_ly: Some(12.0),
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(200.0),
            surface_pressure: Some(0.5),
            gravity: Some(0.2),
            atmosphere: Some("Water".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn exobiology_view_preserves_candidate_values_and_statuses() {
        let view = exobiology_view(&body()).unwrap().unwrap();

        assert!(!view.rows.is_empty());
        assert!(view.rows.iter().any(|row| row.genus == "Оссии"));
        assert!(view.rows.iter().any(|row| row.species == "Discus"));
        assert!(
            view.rows
                .iter()
                .any(|row| row.status.starts_with("Проверить:"))
        );
        assert!(view.maximum_base_value_cr > 0);
        assert!(view.summary[0].contains("Потенциальная базовая выручка"));
        assert!(view.summary[1].contains("First Logged ×5"));
        assert!(view.notes[0].contains("справочник видов неполный"));
    }

    #[test]
    fn confirmed_spansh_genera_limit_the_exobiology_rows() {
        let mut body = body();
        body.biological_signal_count = Some(2);
        body.geological_signal_count = Some(1);
        body.biological_genera = Some(vec!["Osseus".to_owned(), "Tussock".to_owned()]);

        let view = exobiology_view(&body).unwrap().unwrap();
        assert!(view.headline.contains("указанных Spansh"));
        assert!(
            view.rows
                .iter()
                .all(|row| row.genus == "Оссии" || row.genus == "Дерновина")
        );
        assert!(!view.rows.iter().any(|row| row.genus == "Бактерии"));
    }

    #[test]
    fn zero_biological_signals_produce_an_explanatory_view_without_rows() {
        let mut body = body();
        body.biological_signal_count = Some(0);

        let view = exobiology_view(&body).unwrap().unwrap();
        assert!(view.headline.contains("не обнаружил"));
        assert!(view.rows.is_empty());
    }

    #[test]
    fn no_candidates_without_known_genera_returns_none() {
        let mut body = body();
        body.subtype = Some("Icy body".to_owned());
        body.atmosphere = Some("Neon".to_owned());
        body.surface_temperature = Some(40.0);
        body.gravity = Some(1.0);
        body.volcanism_type = Some("No volcanism".to_owned());
        body.geological_signal_count = Some(0);

        assert!(exobiology_view(&body).unwrap().is_none());
    }

    #[test]
    fn display_helpers_localize_groups_and_format_credits() {
        assert_eq!(russian_group_name("Osseus"), "Оссии");
        assert_eq!(russian_group_name("Bacterium"), "Бактерии");
        assert_eq!(format_credits(1_234_567), "1 234 567");
    }
}
