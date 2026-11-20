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

#[derive(Debug, Clone, PartialEq)]
pub struct SearchCriteria {
    pub planet_type: String,
    pub atmosphere_type: String,
    pub temperature: NumericRange,
    pub gravity: NumericRange,
    pub pressure: NumericRange,
    pub count: usize,
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
        if count == 0 {
            return Err(CriteriaError::Count);
        }

        Ok(Self {
            planet_type: normalize_planet_type(planet_type)?,
            atmosphere_type: normalize_atmosphere_type(atmosphere_type)?,
            temperature,
            gravity,
            pressure,
            count,
        })
    }

    pub fn matches(&self, body: &Body) -> bool {
        (self.planet_type == "any"
            || body
                .subtype
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(&self.planet_type)))
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
    }
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
