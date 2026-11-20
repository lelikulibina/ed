use super::body::Body;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericRange {
    pub min: f64,
    pub max: f64,
}

impl NumericRange {
    pub fn new(min: f64, max: f64) -> Option<Self> {
        (min.is_finite() && max.is_finite() && min <= max).then_some(Self { min, max })
    }

    pub fn contains(self, value: Option<f64>) -> bool {
        value.is_some_and(|value| value.is_finite() && value >= self.min && value <= self.max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolcanismFilter {
    Any,
    None,
    WaterMagmaAndGeysers,
}

impl VolcanismFilter {
    pub fn description(self) -> &'static str {
        match self {
            Self::Any => "без ограничений",
            Self::None => "без вулканизма",
            Self::WaterMagmaAndGeysers => "водная магма и гейзеры",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchCriteria {
    pub planet_type: String,
    pub planet_types: Vec<String>,
    pub atmosphere_type: String,
    pub temperature: NumericRange,
    pub gravity: NumericRange,
    pub pressure: NumericRange,
    pub count: usize,
    pub exclude_with_bio_signals: bool,
    pub volcanism: VolcanismFilter,
}

impl SearchCriteria {
    pub fn from_cli(
        planet_type: &str,
        atmosphere_type: &str,
        temperature: NumericRange,
        gravity: NumericRange,
        pressure: NumericRange,
        count: usize,
    ) -> Result<Self, CriteriaError> {
        Self::from_types(
            &[planet_type],
            atmosphere_type,
            temperature,
            gravity,
            pressure,
            count,
        )
    }

    pub fn from_types(
        planet_types: &[&str],
        atmosphere_type: &str,
        temperature: NumericRange,
        gravity: NumericRange,
        pressure: NumericRange,
        count: usize,
    ) -> Result<Self, CriteriaError> {
        if count == 0 {
            return Err(CriteriaError::Count);
        }
        let mut normalized = Vec::new();
        for value in planet_types {
            let value = normalize_planet_type(value)?;
            if value != "any" && !normalized.contains(&value) {
                normalized.push(value);
            }
        }
        if normalized.is_empty() {
            normalized.push("any".to_owned());
        }
        let planet_type = if normalized.len() == 1 {
            normalized[0].clone()
        } else {
            "multiple".to_owned()
        };
        Ok(Self {
            planet_type,
            planet_types: normalized,
            atmosphere_type: normalize_atmosphere_type(atmosphere_type)?,
            temperature,
            gravity,
            pressure,
            count,
            exclude_with_bio_signals: false,
            volcanism: VolcanismFilter::Any,
        })
    }

    pub fn with_bio_signal_exclusion(mut self, exclude: bool) -> Self {
        self.exclude_with_bio_signals = exclude;
        self
    }

    pub fn with_volcanism_filter(mut self, volcanism: VolcanismFilter) -> Self {
        self.volcanism = volcanism;
        self
    }

    pub fn matches(&self, body: &Body) -> bool {
        (self.planet_types.iter().any(|kind| kind == "any")
            || body.subtype.as_deref().is_some_and(|value| {
                self.planet_types
                    .iter()
                    .any(|kind| value.eq_ignore_ascii_case(kind))
            }))
            && (self.atmosphere_type == "any"
                || body
                    .atmosphere
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(&self.atmosphere_type)))
            && self.temperature.contains(body.surface_temperature)
            && self.gravity.contains(body.gravity)
            && self.pressure.contains(body.surface_pressure)
            && body.is_landable == Some(true)
            && body.is_mapped != Some(true)
            && volcanism_matches(self.volcanism, body.volcanism_type.as_deref())
            && (!self.exclude_with_bio_signals || !has_biological_evidence(body))
    }
}

fn volcanism_matches(filter: VolcanismFilter, value: Option<&str>) -> bool {
    match filter {
        VolcanismFilter::Any => true,
        // Exact value present in the repository's Spansh body fixtures.
        VolcanismFilter::None => {
            value.is_some_and(|v| v.trim().eq_ignore_ascii_case("No volcanism"))
        }
        // Elite Dangerous uses Water Magma and Water Geysers as separate
        // volcanism classes, each optionally prefixed with Minor or Major.
        // The profile accepts either water-based class.
        VolcanismFilter::WaterMagmaAndGeysers => value.is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().split_whitespace().collect::<Vec<_>>().as_slice(),
                ["water", "magma"]
                    | ["minor", "water", "magma"]
                    | ["major", "water", "magma"]
                    | ["water", "geysers"]
                    | ["minor", "water", "geysers"]
                    | ["major", "water", "geysers"]
            )
        }),
    }
}

fn has_biological_evidence(body: &Body) -> bool {
    body.biological_signal_count.is_some_and(|count| count > 0)
        || body
            .biological_genera
            .as_ref()
            .is_some_and(|genera| !genera.is_empty())
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CriteriaError {
    #[error("неизвестный тип поверхности планеты: {0}")]
    PlanetType(String),
    #[error("неизвестный тип атмосферы: {0}")]
    AtmosphereType(String),
    #[error("количество результатов должно быть больше нуля")]
    Count,
}

fn normalize_planet_type(value: &str) -> Result<String, CriteriaError> {
    let normalized = value.trim().to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "" | "any" => "any",
        "rocky body" | "rocky" | "rock" => "Rocky body",
        "icy body" | "icy" | "ice" => "Icy body",
        "rocky ice world" | "rocky-ice world" | "rocky ice" => "Rocky ice world",
        "high metal content world" | "high-metal content" | "high metal" => {
            "High metal content world"
        }
        _ => return Err(CriteriaError::PlanetType(value.to_owned())),
    };
    Ok(canonical.to_owned())
}

fn normalize_atmosphere_type(value: &str) -> Result<String, CriteriaError> {
    let normalized = value.trim().to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "" | "any" => "any",
        "ammonia" => "Ammonia",
        "water" => "Water",
        "carbon dioxide" | "carbon-dioxide" | "carbondioxide" => "Carbon dioxide",
        "sulphur dioxide" | "sulphur-dioxide" | "sulfur dioxide" | "sulfur-dioxide"
        | "sulfurdioxide" => "Sulphur dioxide",
        "nitrogen" => "Nitrogen",
        "water-rich" | "water rich" => "Water-rich",
        "methane-rich" | "methan-rich" | "methane rich" => "Methane-rich",
        "carbon dioxide-rich" | "carbon-dioxide rich" | "carbon dioxide rich" => {
            "Carbon dioxide-rich"
        }
        "methane" | "methan" => "Methane",
        "helium" => "Helium",
        "silicate vapour" | "silicate vapor" => "Silicate vapour",
        "neon-rich" | "neon rich" => "Neon-rich",
        "argon-rich" | "argon rich" => "Argon-rich",
        "neon" => "Neon",
        "argon" => "Argon",
        "oxygen" => "Oxygen",
        _ => return Err(CriteriaError::AtmosphereType(value.to_owned())),
    };
    Ok(canonical.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn criteria() -> SearchCriteria {
        SearchCriteria::from_cli(
            "Rocky body",
            "Nitrogen",
            NumericRange {
                min: 200.0,
                max: 300.0,
            },
            NumericRange { min: 0.5, max: 1.5 },
            NumericRange { min: 0.0, max: 2.0 },
            5,
        )
        .unwrap()
    }

    #[test]
    fn range_boundaries_are_inclusive_and_missing_values_do_not_match() {
        let range = NumericRange {
            min: 10.0,
            max: 20.0,
        };
        assert!(range.contains(Some(10.0)));
        assert!(range.contains(Some(20.0)));
        assert!(!range.contains(None));
        assert!(!range.contains(Some(20.01)));
        assert!(NumericRange::new(1.0, 0.0).is_none());
        assert!(NumericRange::new(f64::NAN, 1.0).is_none());
    }

    #[test]
    fn normalizes_legacy_cli_values() {
        assert_eq!(normalize_planet_type("rocky").unwrap(), "Rocky body");
        assert_eq!(
            normalize_atmosphere_type("carbon-dioxide").unwrap(),
            "Carbon dioxide"
        );
        assert_eq!(normalize_atmosphere_type("any").unwrap(), "any");
    }

    #[test]
    fn accepts_every_legacy_planet_and_atmosphere_value() {
        for value in [
            "any",
            "rocky body",
            "icy body",
            "rocky ice world",
            "high metal content world",
        ] {
            assert!(
                normalize_planet_type(value).is_ok(),
                "legacy planet type must be accepted: {value}"
            );
        }

        for value in [
            "any",
            "ammonia",
            "water",
            "carbon-dioxide",
            "sulphur dioxide",
            "sulphur-dioxide",
            "nitrogen",
            "water-rich",
            "methane-rich",
            "carbon dioxide-rich",
            "methane",
            "helium",
            "silicate vapour",
            "neon-rich",
            "argon-rich",
            "neon",
            "argon",
            "oxygen",
        ] {
            assert!(
                normalize_atmosphere_type(value).is_ok(),
                "legacy atmosphere type must be accepted: {value}"
            );
        }
    }

    #[test]
    fn requires_body_to_match_all_legacy_filters() {
        let body = Body {
            id64: Some("1".to_owned()),
            name: "Sol 3".to_owned(),
            system_id64: Some("2".to_owned()),
            system_name: Some("Sol".to_owned()),
            subtype: Some("Rocky body".to_owned()),
            distance_ly: Some(1.0),
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(250.0),
            surface_pressure: Some(1.0),
            gravity: Some(1.0),
            atmosphere: Some("Nitrogen".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        };
        assert!(criteria().matches(&body));
    }

    #[test]
    fn exclude_with_bio_signals_filters_positive_signal_count_and_known_genera() {
        let mut criteria = criteria().with_bio_signal_exclusion(true);
        let mut body = Body {
            id64: None,
            name: "Bio world".to_owned(),
            system_id64: None,
            system_name: None,
            subtype: Some("Rocky body".to_owned()),
            distance_ly: None,
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(250.0),
            surface_pressure: Some(1.0),
            gravity: Some(1.0),
            atmosphere: Some("Nitrogen".to_owned()),
            volcanism_type: None,
            biological_signal_count: Some(2),
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        };
        assert!(!criteria.matches(&body));

        body.biological_signal_count = Some(0);
        body.biological_genera = Some(vec!["Tussock".to_owned()]);
        assert!(!criteria.matches(&body));

        body.biological_genera = Some(vec![]);
        assert!(criteria.matches(&body));

        body.biological_signal_count = None;
        body.biological_genera = None;
        assert!(criteria.matches(&body));

        criteria.exclude_with_bio_signals = false;
        body.biological_signal_count = Some(3);
        assert!(criteria.matches(&body));
    }

    #[test]
    fn multi_subtype_accepts_both_allowed_values() {
        let criteria = SearchCriteria::from_types(
            &["Rocky body", "High metal content world"],
            "Nitrogen",
            NumericRange {
                min: 200.0,
                max: 300.0,
            },
            NumericRange { min: 0.5, max: 1.5 },
            NumericRange { min: 0.0, max: 2.0 },
            5,
        )
        .unwrap();
        let mut body = Body {
            id64: None,
            name: "X".to_owned(),
            system_id64: None,
            system_name: None,
            subtype: Some("Rocky body".to_owned()),
            distance_ly: None,
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(250.0),
            surface_pressure: Some(1.0),
            gravity: Some(1.0),
            atmosphere: Some("Nitrogen".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        };
        assert!(criteria.matches(&body));
        body.subtype = Some("High metal content world".to_owned());
        assert!(criteria.matches(&body));
        body.subtype = Some("Icy body".to_owned());
        assert!(!criteria.matches(&body));
    }

    #[test]
    fn restricted_volcanism_filters_fail_closed_for_unknown_values() {
        for value in [None, Some(""), Some("unknown"), Some("Minor Rocky Magma")] {
            let mut body = Body {
                id64: None,
                name: "X".to_owned(),
                system_id64: None,
                system_name: None,
                subtype: Some("Rocky body".to_owned()),
                distance_ly: None,
                is_landable: Some(true),
                is_mapped: Some(false),
                last_updated_at: None,
                surface_temperature: Some(250.0),
                surface_pressure: Some(1.0),
                gravity: Some(1.0),
                atmosphere: Some("Nitrogen".to_owned()),
                volcanism_type: value.map(str::to_owned),
                biological_signal_count: None,
                geological_signal_count: None,
                biological_genera: None,
                signals_updated_at: None,
            };
            assert!(
                !criteria()
                    .with_volcanism_filter(VolcanismFilter::None)
                    .matches(&body)
            );
            assert!(
                !criteria()
                    .with_volcanism_filter(VolcanismFilter::WaterMagmaAndGeysers)
                    .matches(&body)
            );
            body.volcanism_type = Some("No volcanism".to_owned());
            assert!(
                criteria()
                    .with_volcanism_filter(VolcanismFilter::None)
                    .matches(&body)
            );
        }
    }

    #[test]
    fn mapped_or_non_landable_bodies_are_excluded() {
        let mut body = Body {
            id64: None,
            name: "X".to_owned(),
            system_id64: None,
            system_name: None,
            subtype: Some("Rocky body".to_owned()),
            distance_ly: None,
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(250.0),
            surface_pressure: Some(1.0),
            gravity: Some(1.0),
            atmosphere: Some("Nitrogen".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        };
        assert!(criteria().matches(&body));
        body.is_mapped = Some(true);
        assert!(!criteria().matches(&body));
        body.is_mapped = Some(false);
        body.is_landable = Some(false);
        assert!(!criteria().matches(&body));
    }
}
