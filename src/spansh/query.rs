use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coordinates {
    pub fn validate(self) -> Result<Self, QueryError> {
        if [self.x, self.y, self.z]
            .iter()
            .all(|value| value.is_finite())
        {
            Ok(self)
        } else {
            Err(QueryError::Coordinates)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodySearchQuery {
    pub reference_coords: Coordinates,
    pub radius_ly: f64,
    pub page: u32,
    pub page_size: u16,
}

impl BodySearchQuery {
    pub fn new(
        reference_coords: Coordinates,
        radius_ly: f64,
        page_size: u16,
    ) -> Result<Self, QueryError> {
        reference_coords.validate()?;
        if !radius_ly.is_finite() || radius_ly <= 0.0 {
            return Err(QueryError::Radius);
        }
        if page_size == 0 {
            return Err(QueryError::PageSize);
        }

        Ok(Self {
            reference_coords,
            radius_ly,
            page: 0,
            page_size,
        })
    }

    /// Формирует запрос с серверными фильтрами Spansh. Локальная проверка
    /// остаётся в BodySearch как защита от расхождений API и нормализации.
    pub fn to_payload_with_criteria(
        &self,
        criteria: &crate::domain::criteria::SearchCriteria,
    ) -> Value {
        let mut payload = self.to_payload();
        let filters = payload["filters"]
            .as_object_mut()
            .expect("base query always contains an object of filters");

        if !criteria.planet_types.iter().any(|kind| kind == "any") {
            filters.insert("subtype".to_owned(), json!({ "value": criteria.planet_types }));
        }
        if criteria.atmosphere_type != "any" {
            filters.insert(
                "atmosphere".to_owned(),
                json!({
                    "value": crate::domain::body::atmosphere_filter_labels(
                        &criteria.atmosphere_type
                    )
                }),
            );
        }

        filters.insert(
            "surface_temperature".to_owned(),
            numeric_range_filter(criteria.temperature.min, criteria.temperature.max),
        );
        filters.insert(
            "surface_pressure".to_owned(),
            numeric_range_filter(criteria.pressure.min, criteria.pressure.max),
        );
        filters.insert(
            "gravity".to_owned(),
            numeric_range_filter(criteria.gravity.min, criteria.gravity.max),
        );
        filters.insert("is_landable".to_owned(), json!({ "value": true }));

        payload
    }

    /// Базовый payload для низкоуровневых проверок и контрактных probes.
    /// Рабочий поиск должен использовать to_payload_with_criteria.
    pub fn to_payload(&self) -> Value {
        json!({
            "filters": {
                "distance": { "min": 0, "max": self.radius_ly }
            },
            "sort": [{ "distance": { "direction": "asc" } }],
            "size": self.page_size,
            "page": self.page,
            "reference_coords": {
                "x": self.reference_coords.x,
                "y": self.reference_coords.y,
                "z": self.reference_coords.z
            }
        })
    }
}

fn numeric_range_filter(min: f64, max: f64) -> Value {
    json!({
        "comparison": "<=>",
        "value": [min, max]
    })
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum QueryError {
    #[error("координаты системы должны быть конечными числами")]
    Coordinates,
    #[error("радиус поиска должен быть конечным числом больше нуля")]
    Radius,
    #[error("размер страницы должен быть больше нуля")]
    PageSize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_contains_reference_coordinates_and_radius() {
        let query = BodySearchQuery::new(
            Coordinates {
                x: 55.7,
                y: 17.5,
                z: 27.1,
            },
            50.0,
            100,
        )
        .unwrap();

        let payload = query.to_payload();
        assert_eq!(payload["reference_coords"]["x"], 55.7);
        assert_eq!(payload["reference_coords"]["y"], 17.5);
        assert_eq!(payload["reference_coords"]["z"], 27.1);
        assert_eq!(payload["filters"]["distance"]["min"], 0);
        assert_eq!(payload["filters"]["distance"]["max"], 50.0);
        assert_eq!(payload["page"], 0);
        assert_eq!(payload["size"], 100);
        assert_eq!(payload["sort"][0]["distance"]["direction"], "asc");
    }

    #[test]
    fn payload_pushes_all_body_criteria_to_spansh() {
        use crate::domain::criteria::{NumericRange, SearchCriteria};

        let query = BodySearchQuery::new(
            Coordinates {
                x: 55.7,
                y: 17.5,
                z: 27.1,
            },
            1_000.0,
            100,
        )
        .unwrap();
        let criteria = SearchCriteria::from_cli(
            "Rock",
            "Water",
            NumericRange {
                min: 190.0,
                max: 195.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.042,
                max: 0.237,
            },
            15,
        )
        .unwrap();

        let payload = query.to_payload_with_criteria(&criteria);
        assert_eq!(payload["filters"]["distance"]["max"], 1_000.0);
        assert_eq!(
            payload["filters"]["subtype"]["value"],
            serde_json::json!(["Rocky body"])
        );
        let atmosphere_values = payload["filters"]["atmosphere"]["value"]
            .as_array()
            .unwrap();
        assert!(atmosphere_values.contains(&serde_json::json!("Water")));
        assert!(atmosphere_values.contains(&serde_json::json!("Thin Water")));
        assert!(atmosphere_values.contains(&serde_json::json!("Hot thick Water")));
        assert_eq!(
            payload["filters"]["surface_temperature"]["comparison"],
            "<=>"
        );
        assert_eq!(
            payload["filters"]["surface_temperature"]["value"],
            serde_json::json!([190.0, 195.0])
        );
        assert_eq!(
            payload["filters"]["surface_pressure"]["value"],
            serde_json::json!([0.042, 0.237])
        );
        assert_eq!(
            payload["filters"]["gravity"]["value"],
            serde_json::json!([0.1, 1.0])
        );
        assert_eq!(payload["filters"]["is_landable"]["value"], true);
        assert_eq!(payload["page"], 0);
        assert_eq!(payload["size"], 100);
    }

    #[test]
    fn payload_sends_all_allowed_surface_types() {
        use crate::domain::criteria::{NumericRange, SearchCriteria};
        let query = BodySearchQuery::new(Coordinates { x: 0.0, y: 0.0, z: 0.0 }, 1000.0, 100).unwrap();
        let criteria = SearchCriteria::from_types(
            &["Rocky body", "High metal content world"], "Water",
            NumericRange { min: 395.0, max: 450.0 },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange { min: 0.04, max: 0.063 }, 5,
        ).unwrap();
        let payload = query.to_payload_with_criteria(&criteria);
        assert_eq!(payload["filters"]["subtype"]["value"],
            serde_json::json!(["Rocky body", "High metal content world"]));
    }

    #[test]
    fn rejects_non_finite_coordinates_and_invalid_radius() {
        assert_eq!(
            Coordinates {
                x: f64::NAN,
                y: 0.0,
                z: 0.0
            }
            .validate(),
            Err(QueryError::Coordinates)
        );
        assert_eq!(
            BodySearchQuery::new(
                Coordinates {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                },
                0.0,
                10
            ),
            Err(QueryError::Radius)
        );
        assert_eq!(
            BodySearchQuery::new(
                Coordinates {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                },
                1.0,
                0
            ),
            Err(QueryError::PageSize)
        );
    }
}
