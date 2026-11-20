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
    pub surface_temperature: Option<f64>,
    pub surface_pressure: Option<f64>,
    pub gravity: Option<f64>,
    pub atmosphere: Option<String>,
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

        let name = string_field(value, &["name", "body_name"])
            .ok_or(BodyParseError::MissingName)?;

        Ok(Self {
            id64: id_field(value, &["id64", "body_id64", "id"]),
            name,
            system_id64: id_field(value, &["system_id64", "systemId64"]),
            system_name: string_field(value, &["system_name", "systemName"]),
            subtype: string_field(value, &["subtype", "type"]),
            distance_ly: number_field(value, &["distance", "distance_ly"]),
            is_landable: bool_field(value, &["is_landable", "landable"]),
            is_mapped: bool_field(value, &["is_mapped", "mapped"]),
            surface_temperature: number_field(value, &["surface_temperature", "surface_temp"]),
            surface_pressure: number_field(value, &["surface_pressure", "pressure"]),
            gravity: number_field(value, &["gravity"]),
            atmosphere: string_field(value, &["atmosphere", "atmosphere_type"]),
        })
    }
}

fn id_field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value.get(name).and_then(|field| {
            field.as_str().map(str::to_owned)
                .or_else(|| field.as_u64().map(|id| id.to_string()))
        })
    })
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value.get(name).and_then(Value::as_str).map(str::to_owned)
    })
}

fn number_field(value: &Value, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| {
        value.get(name).and_then(|field| {
            field.as_f64().or_else(|| field.as_str()?.parse().ok())
                .filter(|number| number.is_finite())
        })
    })
}

fn bool_field(value: &Value, names: &[&str]) -> Option<bool> {
    names.iter().find_map(|name| value.get(name).and_then(Value::as_bool))
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
            "surface_temperature": null,
            "surface_pressure": "0.12",
            "gravity": 0.98,
            "atmosphere": "Nitrogen"
        });

        let body = Body::from_json(&value).unwrap();
        assert_eq!(body.id64.as_deref(), Some("9223372036854775807"));
        assert_eq!(body.system_id64.as_deref(), Some("1234567890123456789"));
        assert_eq!(body.surface_temperature, None);
        assert_eq!(body.surface_pressure, Some(0.12));
        assert_eq!(body.gravity, Some(0.98));
        assert_eq!(body.is_mapped, Some(false));
    }

    #[test]
    fn rejects_non_object_and_missing_name() {
        assert_eq!(Body::from_json(&serde_json::json!([])), Err(BodyParseError::NotObject));
        assert_eq!(Body::from_json(&serde_json::json!({"id64":"1"})), Err(BodyParseError::MissingName));
    }
}
