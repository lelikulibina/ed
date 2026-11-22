use super::{
    body::Body,
    catalog::{AtmosphereType, SurfaceType},
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter<T> {
    Any,
    OneOf(Vec<T>),
}

impl<T> Filter<T> {
    pub fn one_of(values: Vec<T>) -> Self {
        if values.is_empty() {
            Self::Any
        } else {
            Self::OneOf(values)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchCriteria {
    pub surface: Filter<SurfaceType>,
    pub atmosphere: Filter<AtmosphereType>,
    pub temperature: NumericRange,
    pub gravity: NumericRange,
    pub pressure: NumericRange,
    pub count: usize,
    pub exclude_with_bio_signals: bool,
}

impl SearchCriteria {
    pub fn new(
        surface: Filter<SurfaceType>,
        atmosphere: Filter<AtmosphereType>,
        temperature: NumericRange,
        gravity: NumericRange,
        pressure: NumericRange,
        count: usize,
    ) -> Result<Self, CriteriaError> {
        if count == 0 {
            return Err(CriteriaError::Count);
        }

        Ok(Self {
            surface,
            atmosphere,
            temperature,
            gravity,
            pressure,
            count,
            exclude_with_bio_signals: false,
        })
    }

    pub fn with_bio_signal_exclusion(mut self, exclude: bool) -> Self {
        self.exclude_with_bio_signals = exclude;
        self
    }

    pub fn matches(&self, body: &Body) -> bool {
        let surface_matches = match &self.surface {
            Filter::Any => true,
            Filter::OneOf(allowed) => body.subtype.as_deref().is_some_and(|value| {
                allowed
                    .iter()
                    .any(|surface| value.eq_ignore_ascii_case(surface.spansh_value()))
            }),
        };
        let atmosphere_matches = match &self.atmosphere {
            Filter::Any => true,
            Filter::OneOf(allowed) => body.atmosphere.as_deref().is_some_and(|value| {
                allowed
                    .iter()
                    .any(|atmosphere| value.eq_ignore_ascii_case(atmosphere.spansh_value()))
            }),
        };

        surface_matches
            && atmosphere_matches
            && self.temperature.contains(body.surface_temperature)
            && self.gravity.contains(body.gravity)
            && self.pressure.contains(body.surface_pressure)
            && body.is_landable == Some(true)
            && body.is_mapped != Some(true)
            && (!self.exclude_with_bio_signals || !has_biological_evidence(body))
    }
}

fn has_biological_evidence(body: &Body) -> bool {
    body.biological_signal_count.is_some_and(|count| count > 0)
        || body
            .biological_genera
            .as_ref()
            .is_some_and(|genera| !genera.is_empty())
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CriteriaError {
    #[error("количество результатов должно быть больше нуля")]
    Count,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn criteria() -> SearchCriteria {
        SearchCriteria::new(
            Filter::one_of(vec![SurfaceType::RockyBody]),
            Filter::one_of(vec![AtmosphereType::Nitrogen]),
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
    fn any_filters_match_bodies_with_unknown_or_missing_surface_and_atmosphere() {
        let base = criteria();
        let criteria = SearchCriteria::new(
            Filter::Any,
            Filter::Any,
            base.temperature,
            base.gravity,
            base.pressure,
            base.count,
        )
        .unwrap();
        let known_values = Body::from_json(&serde_json::json!({
            "name": "Uncatalogued body",
            "subtype": "Metal-rich body",
            "atmosphere": "Xenon",
            "surface_temperature": 250.0,
            "surface_pressure": 1.0,
            "gravity": 1.0,
            "is_landable": true,
            "is_mapped": false
        }))
        .unwrap();
        let missing_values = Body::from_json(&serde_json::json!({
            "name": "Unknown body",
            "surface_temperature": 250.0,
            "surface_pressure": 1.0,
            "gravity": 1.0,
            "is_landable": true,
            "is_mapped": false
        }))
        .unwrap();

        assert!(criteria.matches(&known_values));
        assert!(criteria.matches(&missing_values));
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
    fn alternative_surface_types_match_either_allowed_type() {
        let mut criteria = criteria();
        criteria.surface = Filter::one_of(vec![
            SurfaceType::RockyBody,
            SurfaceType::HighMetalContentWorld,
        ]);

        let mut body = Body {
            id64: None,
            name: "High metal world".to_owned(),
            system_id64: None,
            system_name: None,
            subtype: Some("High metal content world".to_owned()),
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
        body.subtype = Some("Rocky body".to_owned());
        assert!(criteria.matches(&body));
        body.subtype = Some("Icy body".to_owned());
        assert!(!criteria.matches(&body));
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
