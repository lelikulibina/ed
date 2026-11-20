use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub id64: Option<String>,
    pub name: String,
    pub system_id64: Option<String>,
    pub system_name: Option<String>,
    pub subtype: Option<String>,
    pub distance_ly: Option<f64>,
    pub is_landable: Option<bool>,
    pub is_mapped: Option<bool>,
    /// Last known body record update from Spansh; not a First Footfall timestamp.
    pub last_updated_at: Option<String>,
    pub surface_temperature: Option<f64>,
    pub surface_pressure: Option<f64>,
    pub gravity: Option<f64>,
    pub atmosphere: Option<String>,
    pub volcanism_type: Option<String>,
    pub biological_signal_count: Option<u32>,
    pub geological_signal_count: Option<u32>,
    pub biological_genera: Option<Vec<String>>,
    pub signals_updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootfallEvidence {
    /// The latest Spansh body update predates the Odyssey release.
    /// This is a lead only, not proof that First Footfall is available.
    PreOdysseyUpdate,
    /// The body record changed after Odyssey; this does not prove a landing occurred.
    PostOdysseyUpdate,
    Unknown,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum BodyParseError {
    #[error("ожидался JSON-объект небесного тела")]
    NotObject,
    #[error("у небесного тела отсутствует имя")]
    MissingName,
}

impl Body {
    pub fn from_json(value: &Value) -> Result<Self, BodyParseError> {
        if !value.is_object() {
            return Err(BodyParseError::NotObject);
        }
        let root = value
            .get("record")
            .filter(|record| record.is_object())
            .or_else(|| value.get("body").filter(|body| body.is_object()))
            .unwrap_or(value);
        let system = root
            .get("system")
            .or_else(|| value.get("system"))
            .filter(|system| system.is_object());

        let name = string_field(root, &["name", "body_name"]).ok_or(BodyParseError::MissingName)?;

        let (
            biological_signal_count,
            geological_signal_count,
            biological_genera,
            signals_updated_at,
        ) = biological_data(root, value);

        Ok(Self {
            id64: id_field(root, &["id64", "body_id64", "id"]),
            name,
            system_id64: id_field(root, &["system_id64", "systemId64"])
                .or_else(|| {
                    system.and_then(|value| id_field(value, &["id64", "system_id64", "id"]))
                })
                .or_else(|| id_field(value, &["system_id64", "systemId64"])),
            system_name: string_field(root, &["system_name", "systemName"])
                .or_else(|| system.and_then(|value| string_field(value, &["name", "system_name"])))
                .or_else(|| string_field(value, &["system_name", "systemName"])),
            subtype: string_field(root, &["subtype", "subType", "type"])
                .or_else(|| string_field(value, &["subtype", "type"])),
            distance_ly: number_field(root, &["distance", "distance_ly"])
                .or_else(|| number_field(value, &["distance", "distance_ly"])),
            is_landable: bool_field(root, &["is_landable", "isLandable", "landable"])
                .or_else(|| bool_field(value, &["is_landable", "landable"])),
            is_mapped: bool_field(root, &["is_mapped", "mapped"])
                .or_else(|| bool_field(value, &["is_mapped", "mapped"])),
            last_updated_at: string_field(root, &["updated_at", "updateTime"])
                .or_else(|| string_field(value, &["updated_at", "updateTime"])),
            surface_temperature: number_field(
                root,
                &["surface_temperature", "surfaceTemperature", "surface_temp"],
            )
            .or_else(|| number_field(value, &["surface_temperature", "surface_temp"])),
            surface_pressure: number_field(
                root,
                &["surface_pressure", "surfacePressure", "pressure"],
            )
            .or_else(|| number_field(value, &["surface_pressure", "pressure"])),
            gravity: number_field(root, &["gravity"]).or_else(|| number_field(value, &["gravity"])),
            atmosphere: atmosphere_from_values(root, value),
            volcanism_type: string_field(root, &["volcanism_type", "volcanismType"])
                .or_else(|| string_field(value, &["volcanism_type", "volcanismType"])),
            biological_signal_count,
            geological_signal_count,
            biological_genera,
            signals_updated_at,
        })
    }
}

fn biological_data(
    root: &Value,
    value: &Value,
) -> (
    Option<u32>,
    Option<u32>,
    Option<Vec<String>>,
    Option<String>,
) {
    let signal_data = root.get("signals").or_else(|| value.get("signals"));
    let mut biological_count = None;
    let mut geological_count = None;
    let mut genera = None;
    let mut updated_at = string_field(root, &["signals_updated_at", "signalsUpdatedAt"])
        .or_else(|| string_field(value, &["signals_updated_at", "signalsUpdatedAt"]));

    if let Some(signal_data) = signal_data {
        if let Some(signals) = signal_data.as_array() {
            biological_count = Some(sum_signal_count(signals, "Biological"));
            geological_count = Some(sum_signal_count(signals, "Geological"));
        } else if signal_data.is_object() {
            if let Some(signals) = signal_data.get("signals").and_then(Value::as_object) {
                biological_count = Some(
                    signals
                        .get("$SAA_SignalType_Biological;")
                        .and_then(count_value)
                        .unwrap_or(0),
                );
                geological_count = Some(
                    signals
                        .get("$SAA_SignalType_Geological;")
                        .and_then(count_value)
                        .unwrap_or(0),
                );
            }
            genera = signal_data
                .get("genuses")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(normalize_genus_name)
                        .collect()
                });
            updated_at = string_field(signal_data, &["updateTime"]).or(updated_at);
        }
    }

    if genera.is_none() {
        genera = root
            .get("genuses")
            .or_else(|| value.get("genuses"))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .as_str()
                            .or_else(|| entry.get("name").and_then(Value::as_str))
                    })
                    .map(normalize_genus_name)
                    .collect()
            });
    }

    (biological_count, geological_count, genera, updated_at)
}

fn sum_signal_count(signals: &[Value], signal_name: &str) -> u32 {
    signals
        .iter()
        .filter(|signal| {
            signal
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(signal_name))
        })
        .filter_map(|signal| signal.get("count").and_then(count_value))
        .sum()
}

fn count_value(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|count| u32::try_from(count).ok())
        .or_else(|| {
            value.as_f64().and_then(|count| {
                (count.is_finite()
                    && count >= 0.0
                    && count.fract() == 0.0
                    && count <= f64::from(u32::MAX))
                .then_some(count as u32)
            })
        })
}

fn normalize_genus_name(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower.starts_with("$codex_ent_") {
        let token = lower.as_str();
        let mappings = [
            ("$codex_ent_aleoids_genus_name;", "Aleoida"),
            ("$codex_ent_bacterial_genus_name;", "Bacterium"),
            ("$codex_ent_cactoid_genus_name;", "Cactoida"),
            ("$codex_ent_clypeus_genus_name;", "Clypeus"),
            ("$codex_ent_conchas_genus_name;", "Concha"),
            ("$codex_ent_electricae_genus_name;", "Electricae"),
            ("$codex_ent_fonticulus_genus_name;", "Fonticulua"),
            ("$codex_ent_fumerolas_genus_name;", "Fumerola"),
            ("$codex_ent_fungoids_genus_name;", "Fungoida"),
            ("$codex_ent_osseus_genus_name;", "Osseus"),
            ("$codex_ent_recepta_genus_name;", "Recepta"),
            ("$codex_ent_shrubs_genus_name;", "Frutexa"),
            ("$codex_ent_stratum_genus_name;", "Stratum"),
            ("$codex_ent_tubus_genus_name;", "Tubus"),
            ("$codex_ent_tussocks_genus_name;", "Tussock"),
        ];
        if let Some((_, canonical)) = mappings.iter().find(|(symbol, _)| *symbol == token) {
            return (*canonical).to_owned();
        }
        return trimmed.to_owned();
    }

    match lower.as_str() {
        "aleoids" => "Aleoida",
        "bacterial" => "Bacterium",
        "cactoids" => "Cactoida",
        "conchas" => "Concha",
        "fonticulus" => "Fonticulua",
        "fumerolas" => "Fumerola",
        "fungoids" => "Fungoida",
        "shrubs" => "Frutexa",
        "tussocks" => "Tussock",
        _ => trimmed,
    }
    .to_owned()
}

fn atmosphere_from_values(root: &Value, value: &Value) -> Option<String> {
    let label = string_field(root, &["atmosphere", "atmosphere_type", "atmosphereType"])
        .or_else(|| string_field(value, &["atmosphere", "atmosphere_type"]));

    if let Some(label) = label.as_deref()
        && !label.eq_ignore_ascii_case("No atmosphere")
    {
        return Some(normalize_atmosphere_label(label));
    }

    let composition = root
        .get("atmosphere_composition")
        .and_then(Value::as_array)
        .or_else(|| {
            value
                .get("atmosphere_composition")
                .and_then(Value::as_array)
        });
    let dominant_component = composition.and_then(|components| {
        components
            .iter()
            .filter_map(|component| {
                let name = component.get("name").and_then(Value::as_str)?;
                let share = component
                    .get("share")
                    .and_then(Value::as_f64)
                    .filter(|share| share.is_finite())?;
                Some((share, name))
            })
            .max_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, name)| normalize_atmosphere_label(name))
    });

    dominant_component.or_else(|| label.map(|label| normalize_atmosphere_label(&label)))
}

/// Spansh stores atmosphere thickness/temperature as part of the label, while the
/// CLI intentionally compares the normalized atmosphere family. Query all known
/// label variants that collapse to the same family so server and local filters agree.
pub fn atmosphere_filter_labels(value: &str) -> Vec<String> {
    let prefixes = ["", "Thin ", "Thick ", "Hot ", "Hot thin ", "Hot thick "];
    let mut candidates = prefixes
        .iter()
        .map(|prefix| format!("{prefix}{value}"))
        .collect::<Vec<_>>();

    candidates.extend(
        [
            "Ammonia and Oxygen",
            "Thick Ammonia and Oxygen",
            "Suitable for water-based life",
            "Thick Suitable for water-based life",
            "Thick No atmosphere",
        ]
        .into_iter()
        .map(str::to_owned),
    );

    candidates
        .retain(|candidate| normalize_atmosphere_label(candidate).eq_ignore_ascii_case(value));
    candidates.sort();
    candidates.dedup();
    candidates
}

fn normalize_atmosphere_label(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace('-', " ");
    let patterns = [
        ("no atmosphere", "No atmosphere"),
        ("carbon dioxide rich", "Carbon dioxide-rich"),
        ("water rich", "Water-rich"),
        ("methane rich", "Methane-rich"),
        ("neon rich", "Neon-rich"),
        ("argon rich", "Argon-rich"),
        ("silicate vapour", "Silicate vapour"),
        ("silicate vapor", "Silicate vapour"),
        ("sulphur dioxide", "Sulphur dioxide"),
        ("sulfur dioxide", "Sulphur dioxide"),
        ("carbon dioxide", "Carbon dioxide"),
        ("ammonia", "Ammonia"),
        ("nitrogen", "Nitrogen"),
        ("methane", "Methane"),
        ("helium", "Helium"),
        ("neon", "Neon"),
        ("argon", "Argon"),
        ("oxygen", "Oxygen"),
        ("water", "Water"),
    ];

    patterns
        .iter()
        .find_map(|(needle, canonical)| normalized.contains(needle).then_some(*canonical))
        .unwrap_or(value.trim())
        .to_owned()
}

fn id_field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value.get(name).and_then(|field| {
            field
                .as_str()
                .map(str::to_owned)
                .or_else(|| field.as_u64().map(|id| id.to_string()))
        })
    })
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(Value::as_str).map(str::to_owned))
}

fn number_field(value: &Value, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| {
        value.get(name).and_then(|field| {
            field
                .as_f64()
                .or_else(|| field.as_str()?.parse().ok())
                .filter(|number| number.is_finite())
        })
    })
}

fn bool_field(value: &Value, names: &[&str]) -> Option<bool> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(Value::as_bool))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_body_without_losing_large_ids_or_nullable_fields() {
        let value = serde_json::json!({
            "name": "Sol 3",
            "id64": "9223372036854775807",
            "system_id64": 1234567890123456789_u64,
            "system_name": "Sol",
            "subtype": "Earth-like world",
            "distance": 12.5,
            "is_landable": true,
            "is_mapped": false,
            "updated_at": "2021-05-18T23:59:59Z",
            "surface_temperature": null,
            "surface_pressure": "0.12",
            "gravity": 0.98,
            "atmosphere": "Hot thick Carbon dioxide"
        });

        let body = Body::from_json(&value).unwrap();
        assert_eq!(body.id64.as_deref(), Some("9223372036854775807"));
        assert_eq!(body.system_id64.as_deref(), Some("1234567890123456789"));
        assert_eq!(body.surface_temperature, None);
        assert_eq!(body.surface_pressure, Some(0.12));
        assert_eq!(body.gravity, Some(0.98));
        assert_eq!(body.is_mapped, Some(false));
        assert_eq!(
            body.last_updated_at.as_deref(),
            Some("2021-05-18T23:59:59Z")
        );
        assert_eq!(body.atmosphere.as_deref(), Some("Carbon dioxide"));
    }

    #[test]
    fn parses_nested_parent_system_fields() {
        let body = Body::from_json(&serde_json::json!({
            "body": {
                "name": "A 1",
                "id64": "9007199254740993",
                "subtype": "Rocky body",
                "updateTime": "2026-11-20T00:00:00Z"
            },
            "system": {"id64": "123", "name": "A"},
            "distance": 2.5
        }))
        .unwrap();
        assert_eq!(body.id64.as_deref(), Some("9007199254740993"));
        assert_eq!(body.system_id64.as_deref(), Some("123"));
        assert_eq!(body.system_name.as_deref(), Some("A"));
        assert_eq!(
            body.last_updated_at.as_deref(),
            Some("2026-11-20T00:00:00Z")
        );
        assert_eq!(body.distance_ly, Some(2.5));
    }

    #[test]
    fn uses_dominant_composition_when_spansh_label_says_no_atmosphere() {
        let value = serde_json::json!({
            "name": "Earth",
            "atmosphere": "No atmosphere",
            "atmosphere_composition": [
                {"name": "Argon", "share": 0.93},
                {"name": "Nitrogen", "share": 77.88},
                {"name": "Oxygen", "share": 20.89}
            ]
        });
        let body = Body::from_json(&value).unwrap();
        assert_eq!(body.atmosphere.as_deref(), Some("Nitrogen"));
    }

    #[test]
    fn normalizes_descriptive_spansh_atmosphere_labels() {
        assert_eq!(
            normalize_atmosphere_label("Hot thick Carbon dioxide"),
            "Carbon dioxide"
        );
        assert_eq!(
            normalize_atmosphere_label("Carbon dioxide-rich atmosphere"),
            "Carbon dioxide-rich"
        );
        assert_eq!(normalize_atmosphere_label("Water-rich"), "Water-rich");
        assert_eq!(normalize_atmosphere_label("No atmosphere"), "No atmosphere");
    }

    #[test]
    fn atmosphere_filter_labels_include_descriptive_variants() {
        let labels = atmosphere_filter_labels("Water");
        assert!(labels.contains(&"Water".to_owned()));
        assert!(labels.contains(&"Thin Water".to_owned()));
        assert!(labels.contains(&"Thick Water".to_owned()));
        assert!(labels.contains(&"Hot Water".to_owned()));
        assert!(labels.contains(&"Hot thick Water".to_owned()));
        assert!(!labels.contains(&"Water-rich".to_owned()));
    }

    #[test]
    fn parses_search_body_biological_signals_genuses_and_volcanism() {
        let body = Body::from_json(&serde_json::json!({
            "name": "Drojia 3",
            "subtype": "Rocky body",
            "atmosphere": "Water",
            "volcanism_type": "No volcanism",
            "signals": [
                {"name": "Human", "count": 1},
                {"name": "Biological", "count": 3}
            ],
            "genuses": [{"name": "Tussocks"}, {"name": "Bacterial"}, {"name": "Clypeus"}],
            "signals_updated_at": "2026-11-20T00:00:00Z",
            "updated_at": "2026-11-20T00:01:00Z"
        }))
        .unwrap();

        assert_eq!(body.volcanism_type.as_deref(), Some("No volcanism"));
        assert_eq!(
            body.last_updated_at.as_deref(),
            Some("2026-11-20T00:01:00Z")
        );
        assert_eq!(body.biological_signal_count, Some(3));
        assert_eq!(body.geological_signal_count, Some(0));
        assert_eq!(
            body.biological_genera,
            Some(vec![
                "Tussock".to_owned(),
                "Bacterium".to_owned(),
                "Clypeus".to_owned()
            ])
        );
        assert_eq!(
            body.signals_updated_at.as_deref(),
            Some("2026-11-20T00:00:00Z")
        );
    }

    #[test]
    fn parses_official_body_lookup_record_shape() {
        let body = Body::from_json(&serde_json::json!({
            "record": {
                "id64": 123,
                "system_id64": 456,
                "system_name": "Drojia",
                "name": "Drojia 3",
                "type": "Planet",
                "subtype": "Rocky body",
                "distance": 12.5,
                "atmosphere": "Water",
                "volcanism_type": "No volcanism",
                "surface_temperature": 425.0,
                "surface_pressure": 0.05,
                "gravity": 0.2,
                "is_landable": true,
                "signals": [
                    {"name": "Biological", "count": 3},
                    {"name": "Geological", "count": 1}
                ],
                "signal_count": 4,
                "signals_updated_at": "2026-11-20T00:00:00Z",
                "genuses": [{"name": "Bacterial"}, {"name": "Tussocks"}]
            }
        }))
        .unwrap();

        assert_eq!(body.id64.as_deref(), Some("123"));
        assert_eq!(body.system_id64.as_deref(), Some("456"));
        assert_eq!(body.volcanism_type.as_deref(), Some("No volcanism"));
        assert_eq!(body.biological_signal_count, Some(3));
        assert_eq!(body.geological_signal_count, Some(1));
        assert_eq!(
            body.biological_genera,
            Some(vec!["Bacterium".to_owned(), "Tussock".to_owned()])
        );
    }

    #[test]
    fn parses_dump_body_detail_wrapper_and_codex_genus_tokens() {
        let body = Body::from_json(&serde_json::json!({
            "record": {
                "name": "Drojia 3",
                "subType": "Rocky body",
                "surfaceTemperature": 425.0,
                "surfacePressure": 0.05,
                "isLandable": true,
                "volcanismType": "Minor Rocky Magma",
                "signals": {
                    "signals": {
                        "$SAA_SignalType_Biological;": 2,
                        "$SAA_SignalType_Geological;": 1
                    },
                    "genuses": [
                        "$Codex_Ent_Tussocks_Genus_Name;",
                        "$Codex_Ent_Bacterial_Genus_Name;"
                    ],
                    "updateTime": "2026-11-20T00:00:00Z"
                }
            }
        }))
        .unwrap();

        assert_eq!(body.subtype.as_deref(), Some("Rocky body"));
        assert_eq!(body.surface_temperature, Some(425.0));
        assert_eq!(body.surface_pressure, Some(0.05));
        assert_eq!(body.is_landable, Some(true));
        assert_eq!(body.volcanism_type.as_deref(), Some("Minor Rocky Magma"));
        assert_eq!(body.biological_signal_count, Some(2));
        assert_eq!(body.geological_signal_count, Some(1));
        assert_eq!(
            body.biological_genera,
            Some(vec!["Tussock".to_owned(), "Bacterium".to_owned()])
        );
    }

    #[test]
    fn rejects_non_object_and_missing_name() {
        assert_eq!(
            Body::from_json(&serde_json::json!([])),
            Err(BodyParseError::NotObject)
        );
        assert_eq!(
            Body::from_json(&serde_json::json!({"id64":"1"})),
            Err(BodyParseError::MissingName)
        );
    }
}
