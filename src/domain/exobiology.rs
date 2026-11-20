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
    pub geological_signal_count: Option<u32>,
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
        if let Some(genera) = body
            .biological_genera
            .as_ref()
            .filter(|genera| !genera.is_empty())
        {
            let Some(genus) = entry["genus"].as_str() else {
                continue;
            };
            if !genera
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(genus))
            {
                continue;
            }
        }
        let Some(mut candidate) = evaluate_species(entry, body) else {
            continue;
        };
        if body.biological_signal_count.is_some_and(|count| count > 0)
            && body.biological_genera.as_ref().is_none_or(Vec::is_empty)
        {
            candidate
                .unknown_conditions
                .push("точный список биологических родов Spansh".to_owned());
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
        geological_signal_count: body.geological_signal_count,
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

    if !temperature_rule_matches(entry, body, &mut unknown_conditions) {
        return None;
    }

    if !conditional_temperature_rules_match(entry, body, &mut unknown_conditions) {
        return None;
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

    if entry["terrain"]
        .as_array()
        .is_some_and(|terrain| !terrain.is_empty())
    {
        unknown_conditions.push("тип рельефа".to_owned());
    }

    if entry["requires_geological_signals"].as_bool() == Some(true) {
        match body.geological_signal_count {
            Some(0) => return None,
            Some(_) => {
                unknown_conditions.push("конкретные геологические объекты/фумаролы".to_owned())
            }
            None => unknown_conditions.push("геологические сигналы".to_owned()),
        }
    }

    if let Some(allowed) = entry["volcanism"]
        .as_array()
        .filter(|values| !values.is_empty())
    {
        match body.volcanism_type.as_deref() {
            Some(actual)
                if allowed.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|expected| expected.eq_ignore_ascii_case(actual.trim()))
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

fn conditional_temperature_rules_match(
    entry: &Value,
    body: &Body,
    unknown_conditions: &mut Vec<String>,
) -> bool {
    let Some(rules) = entry["conditional_temperature_rules"].as_array() else {
        return true;
    };

    for rule in rules {
        let Some(atmospheres) = rule["atmospheres"].as_array() else {
            continue;
        };
        match body.atmosphere.as_deref() {
            Some(actual)
                if atmospheres.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|expected| expected.eq_ignore_ascii_case(actual.trim()))
                }) =>
            {
                if !temperature_rule_matches(rule, body, unknown_conditions) {
                    return false;
                }
            }
            Some(_) => {}
            None => unknown_conditions
                .push("атмосфера для условного температурного ограничения".to_owned()),
        }
    }

    true
}

fn temperature_rule_matches(
    rule: &Value,
    body: &Body,
    unknown_conditions: &mut Vec<String>,
) -> bool {
    let minimum = rule["temperature_min_k"].as_f64();
    let maximum = rule["temperature_max_k"].as_f64();
    let Some(actual) = body.surface_temperature else {
        if minimum.is_some() || maximum.is_some() {
            unknown_conditions.push("температура поверхности".to_owned());
        }
        return true;
    };

    if let Some(minimum) = minimum {
        let inclusive = rule["temperature_min_inclusive"].as_bool().unwrap_or(true);
        let outside_range = if inclusive {
            actual < minimum
        } else {
            actual <= minimum
        };
        if outside_range {
            return false;
        }
    }

    if let Some(maximum) = maximum {
        let inclusive = rule["temperature_max_inclusive"].as_bool().unwrap_or(true);
        let outside_range = if inclusive {
            actual > maximum
        } else {
            actual >= maximum
        };
        if outside_range {
            return false;
        }
    }

    true
}

