use super::body::Body;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericRange {
    pub min: f64,
    pub max: f64,
}

impl NumericRange {
    pub fn contains(self, value: Option<f64>) -> bool {
        value.is_some_and(|value| value >= self.min && value <= self.max)
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
    pub fn matches(&self, body: &Body) -> bool {
        (self.planet_type.eq_ignore_ascii_case("any")
            || body.subtype.as_deref().is_some_and(|value| value.eq_ignore_ascii_case(&self.planet_type)))
            && (self.atmosphere_type.eq_ignore_ascii_case("any")
                || body.atmosphere.as_deref().is_some_and(|value| value.eq_ignore_ascii_case(&self.atmosphere_type)))
            && self.temperature.contains(body.surface_temperature)
            && self.gravity.contains(body.gravity)
            && self.pressure.contains(body.surface_pressure)
            && body.is_landable == Some(true)
            && body.is_mapped == Some(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn criteria() -> SearchCriteria {
        SearchCriteria {
            planet_type: "Earth-like world".to_owned(),
            atmosphere_type: "Nitrogen".to_owned(),
            temperature: NumericRange { min: 200.0, max: 300.0 },
            gravity: NumericRange { min: 0.5, max: 1.5 },
            pressure: NumericRange { min: 0.0, max: 2.0 },
            count: 5,
        }
    }

    #[test]
    fn range_boundaries_are_inclusive_and_missing_values_do_not_match() {
        let range = NumericRange { min: 10.0, max: 20.0 };
        assert!(range.contains(Some(10.0)));
        assert!(range.contains(Some(20.0)));
        assert!(!range.contains(None));
        assert!(!range.contains(Some(20.01)));
    }

    #[test]
    fn requires_body_to_match_all_legacy_filters() {
        let body = Body {
            id64: Some("1".to_owned()),
            name: "Sol 3".to_owned(),
            system_id64: Some("2".to_owned()),
            system_name: Some("Sol".to_owned()),
            subtype: Some("Earth-like world".to_owned()),
            distance_ly: Some(1.0),
            is_landable: Some(true),
            is_mapped: Some(false),
            surface_temperature: Some(250.0),
            surface_pressure: Some(1.0),
            gravity: Some(1.0),
            atmosphere: Some("Nitrogen".to_owned()),
        };
        assert!(criteria().matches(&body));
    }

    #[test]
    fn mapped_or_non_landable_bodies_are_excluded() {
        let mut body = Body {
            id64: None, name: "X".to_owned(), system_id64: None, system_name: None,
            subtype: Some("Earth-like world".to_owned()), distance_ly: None,
            is_landable: Some(true), is_mapped: Some(false),
            surface_temperature: Some(250.0), surface_pressure: Some(1.0),
            gravity: Some(1.0), atmosphere: Some("Nitrogen".to_owned()),
        };
        body.is_mapped = Some(true);
        assert!(!criteria().matches(&body));
        body.is_mapped = Some(false);
        body.is_landable = Some(false);
        assert!(!criteria().matches(&body));
    }
}
