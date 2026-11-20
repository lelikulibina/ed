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
                .push("точный список биологических групп Spansh".to_owned());
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
            last_updated_at: None,
            surface_temperature: temperature,
            surface_pressure: None,
            gravity,
            atmosphere: atmosphere.map(str::to_owned),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
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
        assert_eq!(
            estimate.maximum_base_value_cr,
            estimate
                .genera
                .iter()
                .map(|genus| genus.max_species_value_cr)
                .sum::<u64>()
        );
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
        let mut planet = body(Some("Rocky body"), Some("Water"), Some(200.0), Some(0.2));
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
        let mut planet = body(Some("Rocky body"), Some("Water"), Some(200.0), Some(0.2));
        planet.biological_signal_count = Some(0);

        let estimate = estimate(&planet).unwrap();
        assert!(estimate.genera.is_empty());
        assert_eq!(estimate.maximum_base_value_cr, 0);
    }

    #[test]
    fn known_no_volcanism_confirms_bacterium_but_volcanic_world_excludes_it() {
        let mut planet = body(Some("Rocky body"), Some("Water"), Some(250.0), Some(0.5));
        planet.volcanism_type = Some("No volcanism".to_owned());
        let no_volcanism_estimate = estimate(&planet).unwrap();
        let cerbrus = no_volcanism_estimate
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
        assert!(
            !estimate
                .genera
                .iter()
                .any(|genus| genus.genus == "Bacterium")
        );
    }

    #[test]
    fn catalog_covers_species_for_the_profiles_main_atmospheres() {
        let co2_192 = estimate(&body(
            Some("Rocky body"),
            Some("Carbon dioxide"),
            Some(192.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(co2_192.genera.iter().any(|genus| {
            genus.genus == "Aleoida"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Gravis")
        }));
        assert!(co2_192.genera.iter().any(|genus| {
            genus.genus == "Stratum"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Cucumisis")
        }));

        let ammonia_170 = estimate(&body(
            Some("Rocky body"),
            Some("Ammonia"),
            Some(170.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(ammonia_170.genera.iter().any(|genus| {
            genus.genus == "Frutexa"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Flammasis")
        }));
        assert!(ammonia_170.genera.iter().any(|genus| {
            genus.genus == "Aleoida"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Spica")
        }));

        let oxygen_icy = estimate(&body(
            Some("Icy body"),
            Some("Oxygen"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(oxygen_icy.genera.iter().any(|genus| {
            genus.genus == "Fonticulua"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Fluctus")
        }));

        let sulphur_dioxide = estimate(&body(
            Some("Rocky body"),
            Some("Sulphur dioxide"),
            Some(180.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(sulphur_dioxide.genera.iter().any(|genus| {
            genus.genus == "Tussock"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Stigmasis")
        }));
        assert!(sulphur_dioxide.genera.iter().any(|genus| {
            genus.genus == "Recepta"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Deltahedronix")
        }));
    }

    #[test]
    fn fumerola_requires_matching_volcanism_and_unverified_geological_features() {
        let mut planet = body(
            Some("Rocky Ice world"),
            Some("Argon"),
            Some(100.0),
            Some(0.2),
        );
        planet.volcanism_type = Some("Water Magma".to_owned());
        planet.geological_signal_count = Some(1);

        let aquatis_estimate = estimate(&planet).unwrap();
        let aquatis = aquatis_estimate
            .genera
            .iter()
            .find(|genus| genus.genus == "Fumerola")
            .unwrap()
            .candidates
            .iter()
            .find(|candidate| candidate.species == "Aquatis")
            .unwrap();
        assert_eq!(aquatis.status, CandidateStatus::NeedsVerification);
        assert!(
            aquatis
                .unknown_conditions
                .iter()
                .any(|value| value.contains("фумаролы"))
        );
        assert!(
            aquatis
                .unknown_conditions
                .iter()
                .any(|value| value.contains("рельефа"))
        );

        planet.geological_signal_count = Some(0);
        assert!(
            !estimate(&planet)
                .unwrap()
                .genera
                .iter()
                .any(|genus| genus.genus == "Fumerola")
        );
    }

    #[test]
    fn conditional_temperature_rules_only_apply_to_their_atmospheres() {
        let ammonia_hmc = estimate(&body(
            Some("High metal content world"),
            Some("Ammonia"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(ammonia_hmc.genera.iter().any(|genus| {
            genus.genus == "Frutexa"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Metallicum")
        }));

        let co2_hmc = estimate(&body(
            Some("High metal content world"),
            Some("Carbon dioxide"),
            Some(200.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(!co2_hmc.genera.iter().any(|genus| {
            genus.genus == "Frutexa"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Metallicum")
        }));

        let water_150 = estimate(&body(
            Some("Rocky body"),
            Some("Water"),
            Some(150.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(water_150.genera.iter().any(|genus| {
            genus.genus == "Stratum"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Paleas")
        }));

        let ammonia_150 = estimate(&body(
            Some("Rocky body"),
            Some("Ammonia"),
            Some(150.0),
            Some(0.2),
        ))
        .unwrap();
        assert!(!ammonia_150.genera.iter().any(|genus| {
            genus.genus == "Stratum"
                && genus
                    .candidates
                    .iter()
                    .any(|candidate| candidate.species == "Paleas")
        }));
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
