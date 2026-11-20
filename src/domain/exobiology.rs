use std::collections::BTreeMap;

use serde_json::Value;

use super::body::Body;

const SPECIES_CATALOG: &str = include_str!("../../data/exobiology/species-v1.json");
const FIRST_LOGGED_MULTIPLIER: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateStatus {
    EligibleByKnownData,
    NeedsVerification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesCandidate {
    pub genus: String,
    pub species: String,
    pub base_value_cr: u64,
    pub status: CandidateStatus,
    pub unknown_conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenusEstimate {
    pub genus: String,
    pub candidates: Vec<SpeciesCandidate>,
    pub min_species_value_cr: u64,
    pub max_species_value_cr: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExobiologyEstimate {
    pub genera: Vec<GenusEstimate>,
    /// No genus is confirmed until biological signals or observations are known.
    pub minimum_base_value_cr: u64,
    /// Assumes every listed candidate genus can occur; this is a theoretical upper bound.
    pub maximum_base_value_cr: u64,
    pub minimum_first_logged_value_cr: u64,
    pub maximum_first_logged_value_cr: u64,
    pub probabilities_calculated: bool,
    pub biological_signal_count: Option<u32>,
    pub biological_genera: Option<Vec<String>>,
    pub catalog_is_complete: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum EstimateError {
    #[error("справочник экзобиологии имеет неверный формат: {0}")]
    InvalidCatalog(#[from] serde_json::Error),
    #[error("в справочнике отсутствует массив species")]
    MissingSpecies,
}

pub fn estimate(body: &Body) -> Result<ExobiologyEstimate, EstimateError> {
    let catalog: Value = serde_json::from_str(SPECIES_CATALOG)?;
    let entries = catalog["species"]
        .as_array()
        .ok_or(EstimateError::MissingSpecies)?;

    let mut by_genus: BTreeMap<String, Vec<SpeciesCandidate>> = BTreeMap::new();

    for entry in entries {
        if body.biological_signal_count == Some(0) {
            break;
        }
        if let Some(genera) = body.biological_genera.as_ref().filter(|genera| !genera.is_empty()) {
            let Some(genus) = entry["genus"].as_str() else {
                continue;
            };
            if !genera.iter().any(|candidate| candidate.eq_ignore_ascii_case(genus)) {
                continue;
            }
        }
        let Some(mut candidate) = evaluate_species(entry, body) else {
            continue;
        };
        if body.biological_signal_count.is_some_and(|count| count > 0)
            && body.biological_genera.as_ref().is_none_or(Vec::is_empty)
        {
            candidate.unknown_conditions.push("точный список биологических родов Spansh".to_owned());
            candidate.status = CandidateStatus::NeedsVerification;
        }
        by_genus
            .entry(candidate.genus.clone())
            .or_default()
            .push(candidate);
    }

    let genera = by_genus
        .into_iter()
        .map(|(genus, mut candidates)| {
            candidates.sort_by(|left, right| {
                left.species
                    .cmp(&right.species)
                    .then(left.base_value_cr.cmp(&right.base_value_cr))
            });
            let min_species_value_cr = candidates
                .iter()
                .map(|candidate| candidate.base_value_cr)
                .min()
                .unwrap_or_default();
            let max_species_value_cr = candidates
                .iter()
                .map(|candidate| candidate.base_value_cr)
                .max()
                .unwrap_or_default();
            GenusEstimate {
                genus,
                candidates,
                min_species_value_cr,
                max_species_value_cr,
            }
        })
        .collect::<Vec<_>>();

    let maximum_base_value_cr = genera
        .iter()
        .map(|genus| genus.max_species_value_cr)
        .sum::<u64>();

    Ok(ExobiologyEstimate {
        genera,
        minimum_base_value_cr: 0,
        maximum_base_value_cr,
        minimum_first_logged_value_cr: 0,
        maximum_first_logged_value_cr: maximum_base_value_cr * FIRST_LOGGED_MULTIPLIER,
        probabilities_calculated: false,
        biological_signal_count: body.biological_signal_count,
        biological_genera: body.biological_genera.clone(),
        catalog_is_complete: false,
    })
}

fn evaluate_species(entry: &Value, body: &Body) -> Option<SpeciesCandidate> {
    let genus = entry["genus"].as_str()?.to_owned();
    let species = entry["species"].as_str()?.to_owned();
    let base_value_cr = entry["base_value_cr"].as_u64()?;
    let mut unknown_conditions = Vec::new();

    if !match_string_constraint(
        entry["planet_types"].as_array().map(Vec::as_slice),
        body.subtype.as_deref(),
        "тип планеты",
        &mut unknown_conditions,
    ) {
        return None;
    }

    if !match_string_constraint(
        entry["atmospheres"].as_array().map(Vec::as_slice),
        body.atmosphere.as_deref(),
        "атмосфера",
        &mut unknown_conditions,
    ) {
        return None;
    }

    if let Some(minimum) = entry["temperature_min_k"].as_f64() {
        match body.surface_temperature {
            Some(actual) => {
                let inclusive = entry["temperature_min_inclusive"].as_bool().unwrap_or(true);
                let outside_range = if inclusive {
                    actual < minimum
                } else {
                    actual <= minimum
                };
                if outside_range {
                    return None;
                }
            }
            None => unknown_conditions.push("температура поверхности".to_owned()),
        }
    }

    if let Some(maximum) = entry["temperature_max_k"].as_f64() {
        match body.surface_temperature {
            Some(actual) => {
                let inclusive = entry["temperature_max_inclusive"].as_bool().unwrap_or(true);
                let outside_range = if inclusive {
                    actual > maximum
                } else {
                    actual >= maximum
                };
                if outside_range {
                    return None;
                }
            }
            None => unknown_conditions.push("температура поверхности".to_owned()),
        }
    }

    if let Some(maximum) = entry["gravity_max_g"].as_f64() {
        match body.gravity {
            Some(actual) => {
                let inclusive = entry["gravity_max_inclusive"].as_bool().unwrap_or(true);
                let outside_range = if inclusive {
                    actual > maximum
                } else {
                    actual >= maximum
                };
                if outside_range {
                    return None;
                }
            }
            None => unknown_conditions.push("гравитация".to_owned()),
        }
    }

    if let Some(allowed) = entry["volcanism"].as_array().filter(|values| !values.is_empty()) {
        match body.volcanism_type.as_deref() {
            Some(actual) if allowed.iter().any(|value| {
                value.as_str().is_some_and(|expected| expected.eq_ignore_ascii_case(actual.trim()))
            }) => {}
            Some(_) => return None,
            None => unknown_conditions.push("тип вулканизма".to_owned()),
        }
    }

    Some(SpeciesCandidate {
        genus,
        species,
        base_value_cr,
        status: if unknown_conditions.is_empty() {
            CandidateStatus::EligibleByKnownData
        } else {
            CandidateStatus::NeedsVerification
        },
        unknown_conditions,
    })
}

fn match_string_constraint(
    allowed: Option<&[Value]>,
    actual: Option<&str>,
    missing_label: &str,
    unknown_conditions: &mut Vec<String>,
) -> bool {
    let Some(allowed) = allowed else {
        return true;
    };
    if allowed.is_empty() {
        return true;
    }

    let Some(actual) = actual else {
        unknown_conditions.push(missing_label.to_owned());
        return true;
    };

    allowed.iter().any(|value| {
        value
            .as_str()
            .is_some_and(|expected| expected.eq_ignore_ascii_case(actual.trim()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(
        subtype: Option<&str>,
        atmosphere: Option<&str>,
        temperature: Option<f64>,
        gravity: Option<f64>,
    ) -> Body {
        Body {
            id64: Some("1".to_owned()),
            name: "Test 1".to_owned(),
            system_id64: Some("2".to_owned()),
            system_name: Some("Test".to_owned()),
            subtype: subtype.map(str::to_owned),
            distance_ly: None,
            is_landable: Some(true),
            is_mapped: Some(false),
            surface_temperature: temperature,
            surface_pressure: None,
            gravity,
            atmosphere: atmosphere.map(str::to_owned),
            volcanism_type: None,
            biological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn water_rocky_world_matches_only_compatible_high_value_species() {
        let estimate = estimate(&body(
            Some("Rocky body"),
            Some("Water"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();

        let osseus = estimate
            .genera
            .iter()
            .find(|genus| genus.genus == "Osseus")
            .unwrap();
        assert!(
            osseus
                .candidates
                .iter()
                .any(|candidate| candidate.species == "Discus")
        );
        let clypeus = estimate
            .genera
            .iter()
            .find(|genus| genus.genus == "Clypeus")
            .unwrap();
        assert_eq!(clypeus.min_species_value_cr, 8_418_000);
        assert_eq!(clypeus.max_species_value_cr, 11_873_200);
        assert_eq!(estimate.minimum_base_value_cr, 0);
        assert_eq!(estimate.maximum_base_value_cr, 57_014_400);
        assert!(!estimate.probabilities_calculated);
    }

    #[test]
    fn water_rich_is_not_treated_as_plain_water() {
        let estimate = estimate(&body(
            Some("Rocky body"),
            Some("Water-rich"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();

        assert!(estimate.genera.iter().any(|genus| genus.genus == "Osseus"));
        assert!(estimate.genera.iter().any(|genus| genus.genus == "Tussock"));
        assert!(
            !estimate
                .genera
                .iter()
                .any(|genus| genus.genus == "Cactoida")
        );
    }

    #[test]
    fn exclusive_temperature_and_gravity_limits_are_enforced() {
        let at_temperature_limit = estimate(&body(
            Some("Rocky body"),
            Some("Water"),
            Some(190.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(
            !at_temperature_limit
                .genera
                .iter()
                .any(|genus| genus.genus == "Clypeus")
        );

        let at_gravity_limit = estimate(&body(
            Some("Rocky body"),
            Some("Sulphur dioxide"),
            Some(200.0),
            Some(0.27),
        ))
        .unwrap();
        assert!(
            !at_gravity_limit
                .genera
                .iter()
                .any(|genus| genus.genus == "Recepta")
        );
    }

    #[test]
    fn unknown_volcanism_keeps_bacterium_as_unverified_not_confirmed() {
        let estimate = estimate(&body(
            Some("Rocky body"),
            Some("Water"),
            Some(250.0),
            Some(0.5),
        ))
        .unwrap();
        let bacterium = estimate
            .genera
            .iter()
            .find(|genus| genus.genus == "Bacterium")
            .unwrap();
        let cerbrus = bacterium
            .candidates
            .iter()
            .find(|candidate| candidate.species == "Cerbrus")
            .unwrap();
        assert_eq!(cerbrus.status, CandidateStatus::NeedsVerification);
        assert!(
            cerbrus
                .unknown_conditions
                .iter()
                .any(|value| value.contains("вулканизма"))
        );
    }

    #[test]
    fn confirmed_biological_genera_narrow_the_candidate_list() {
        let mut planet = body(
            Some("Rocky body"),
            Some("Water"),
            Some(200.0),
            Some(0.2),
        );
        planet.biological_signal_count = Some(2);
        planet.biological_genera = Some(vec!["Osseus".to_owned(), "Tussock".to_owned()]);

        let estimate = estimate(&planet).unwrap();
        assert_eq!(
            estimate
                .genera
                .iter()
                .map(|genus| genus.genus.as_str())
                .collect::<Vec<_>>(),
            vec!["Osseus", "Tussock"]
        );
        assert_eq!(estimate.maximum_base_value_cr, 27_248_600);
        assert_eq!(estimate.biological_signal_count, Some(2));
        assert!(!estimate.catalog_is_complete);
    }

    #[test]
    fn zero_biological_signals_suppress_all_species_candidates() {
        let mut planet = body(
            Some("Rocky body"),
            Some("Water"),
            Some(200.0),
            Some(0.2),
        );
        planet.biological_signal_count = Some(0);

        let estimate = estimate(&planet).unwrap();
        assert!(estimate.genera.is_empty());
        assert_eq!(estimate.maximum_base_value_cr, 0);
    }

    #[test]
    fn known_no_volcanism_confirms_bacterium_but_volcanic_world_excludes_it() {
        let mut planet = body(
            Some("Rocky body"),
            Some("Water"),
            Some(250.0),
            Some(0.5),
        );
        planet.volcanism_type = Some("No volcanism".to_owned());
        let estimate = estimate(&planet).unwrap();
        let cerbrus = estimate
            .genera
            .iter()
            .find(|genus| genus.genus == "Bacterium")
            .unwrap()
            .candidates
            .iter()
            .find(|candidate| candidate.species == "Cerbrus")
            .unwrap();
        assert_eq!(cerbrus.status, CandidateStatus::EligibleByKnownData);

        planet.volcanism_type = Some("Minor Rocky Magma".to_owned());
        let estimate = estimate(&planet).unwrap();
        assert!(!estimate.genera.iter().any(|genus| genus.genus == "Bacterium"));
    }

    #[test]
    fn first_logged_is_reported_as_a_separate_five_times_scenario() {
        let estimate = estimate(&body(
            Some("Rocky body"),
            Some("Water"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();
        assert_eq!(
            estimate.maximum_first_logged_value_cr,
            estimate.maximum_base_value_cr * 5
        );
        assert_eq!(estimate.minimum_first_logged_value_cr, 0);
    }
}
