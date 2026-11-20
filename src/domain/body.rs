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
        let root = value.get("body").filter(|body| body.is_object()).unwrap_or(value);
        let system = root.get("system").filter(|system| system.is_object());

        let name = string_field(root, &["name", "body_name"])
            .ok_or(BodyParseError::MissingName)?;

        Ok(Self {
            id64: id_field(root, &["id64", "body_id64", "id"]),
            name,
            system_id64: id_field(root, &["system_id64", "systemId64"])
                .or_else(|| system.and_then(|value| id_field(value, &["id64", "system_id64", "id"]))),
            system_name: string_field(root, &["system_name", "systemName"])
                .or_else(|| system.and_then(|value| string_field(value, &["name", "system_name"]))),
            subtype: string_field(root, &["subtype", "type"]),
            distance_ly: number_field(root, &["distance", "distance_ly"]),
            is_landable: bool_field(root, &["is_landable", "landable"]),
            is_mapped: bool_field(root, &["is_mapped", "mapped"]),
            surface_temperature: number_field(root, &["surface_temperature", "surface_temp"]),
            surface_pressure: number_field(root, &["surface_pressure", "pressure"]),
            gravity: number_field(root, &["gravity"]),
            atmosphere: string_field(root, &["atmosphere", "atmosphere_type"]),
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
    fn parses_nested_parent_system_fields() {
        let body = Body::from_json(&serde_json::json!({
            "body": {
                "name": "A 1",
                "id64": "9007199254740993",
                "subtype": "Rocky body",
                "system": {"id64": "123", "name": "A"},
                "distance": 2.5
            }
        })).unwrap();
        assert_eq!(body.id64.as_deref(), Some("9007199254740993"));
        assert_eq!(body.system_id64.as_deref(), Some("123"));
        assert_eq!(body.system_name.as_deref(), Some("A"));
        assert_eq!(body.distance_ly, Some(2.5));
    }

    #[test]
    fn rejects_non_object_and_missing_name() {
        assert_eq!(Body::from_json(&serde_json::json!([])), Err(BodyParseError::NotObject));
        assert_eq!(Body::from_json(&serde_json::json!({"id64":"1"})), Err(BodyParseError::MissingName));
    }
}
