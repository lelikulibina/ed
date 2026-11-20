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
            Err(QueryError::InvalidCoordinates)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodySearchQuery {
    pub reference_coords: Coordinates,
    pub radius_ly: f64,
    pub page: u32,
    pub page_size: u16,
    pub subtypes: Vec<String>,
    pub landable_only: Option<bool>,
}

impl BodySearchQuery {
    pub fn new(
        reference_coords: Coordinates,
        radius_ly: f64,
        page_size: u16,
    ) -> Result<Self, QueryError> {
        reference_coords.validate()?;
        if !radius_ly.is_finite() || radius_ly <= 0.0 {
            return Err(QueryError::InvalidRadius);
        }
        if page_size == 0 {
            return Err(QueryError::InvalidPageSize);
        }

        Ok(Self {
            reference_coords,
            radius_ly,
            page: 0,
            page_size,
            subtypes: Vec::new(),
            landable_only: None,
        })
    }

    pub fn with_subtypes(mut self, subtypes: impl IntoIterator<Item = String>) -> Self {
        self.subtypes = subtypes.into_iter().collect();
        self
    }

    pub fn landable_only(mut self, enabled: bool) -> Self {
        self.landable_only = Some(enabled);
        self
    }

    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Формирует предварительный Spansh payload. Поля должны быть сверены
    /// с живым ответом API и сохранёнными fixtures до production-переключения.
    pub fn to_payload(&self) -> Value {
        let mut filters = serde_json::Map::new();
        filters.insert(
            "distance".to_owned(),
            json!({ "min": 0, "max": self.radius_ly }),
        );

        if !self.subtypes.is_empty() {
            filters.insert("subtype".to_owned(), json!({ "value": self.subtypes }));
        }
        if let Some(landable) = self.landable_only {
            filters.insert("is_landable".to_owned(), json!({ "value": landable }));
        }

        json!({
            "filters": Value::Object(filters),
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

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum QueryError {
    #[error("координаты системы должны быть конечными числами")]
    InvalidCoordinates,
    #[error("радиус поиска должен быть конечным числом больше нуля")]
    InvalidRadius,
    #[error("размер страницы должен быть больше нуля")]
    InvalidPageSize,
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
        .unwrap()
        .with_subtypes(vec!["Earth-like world".to_owned()])
        .landable_only(true);

        let payload = query.to_payload();
        assert_eq!(payload["reference_coords"]["x"], 55.7);
        assert_eq!(payload["filters"]["distance"]["max"], 50.0);
        assert_eq!(
            payload["filters"]["subtype"]["value"][0],
            "Earth-like world"
        );
        assert!(payload["filters"]["is_landable"]["value"].as_bool().unwrap());
        assert_eq!(payload["page"], 0);
        assert_eq!(payload["size"], 100);
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
            Err(QueryError::InvalidCoordinates)
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
            Err(QueryError::InvalidRadius)
        );
    }
}
