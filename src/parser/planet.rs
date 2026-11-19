use anyhow::{Context, Result};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PlanetInfo {
    pub name: String,
    pub system_name: String,
    /// None means the data source does not publish a mapping status.
    pub is_mapped: Option<bool>,
    pub surface_temp: Option<i32>,
    /// Gravity in Earth gravities (g), as returned by Spansh.
    pub gravity: Option<f32>,
    pub atmosphere: Option<String>,
    /// Surface pressure in atmospheres, as returned by Spansh.
    pub pressure: Option<f32>,
}

/// Converts a Spansh body-search result into the values used by the CLI.
pub fn parse_spansh_planet(body: &Value) -> Result<Option<PlanetInfo>> {
    if body.get("type").and_then(Value::as_str) != Some("Planet")
        || body.get("is_landable").and_then(Value::as_bool) != Some(true)
    {
        return Ok(None);
    }

    let name = body
        .get("name")
        .and_then(Value::as_str)
        .context("Spansh вернул пригодную для посадки планету без имени")?
        .to_string();
    let system_name = body
        .get("system_name")
        .and_then(Value::as_str)
        .context("Spansh вернул планету без имени звёздной системы")?
        .to_string();

    let gravity = body
        .get("gravity")
        .and_then(Value::as_f64)
        .map(|value| value as f32);
    let pressure = body
        .get("surface_pressure")
        .and_then(Value::as_f64)
        .map(|value| value as f32);
    let surface_temp = body
        .get("surface_temperature")
        .and_then(Value::as_f64)
        .map(|value| value as i32);
    let atmosphere = body
        .get("atmosphere")
        .and_then(Value::as_str)
        .map(str::to_string);
    let is_mapped = body.get("is_mapped").and_then(Value::as_bool);

    Ok(Some(PlanetInfo {
        name,
        system_name,
        is_mapped,
        surface_temp,
        gravity,
        atmosphere,
        pressure,
    }))
}

#[cfg(test)]
mod tests {
    use super::parse_spansh_planet;
    use serde_json::json;

    #[test]
    fn parses_spansh_body_values_without_unit_conversion() {
        let body = json!({
            "type": "Planet",
            "name": "Mercury",
            "system_name": "Sol",
            "subtype": "Metal-rich body",
            "is_landable": true,
            "gravity": 0.375546,
            "surface_temperature": 401.965271,
            "surface_pressure": 0.0,
            "atmosphere": "No atmosphere"
        });

        let planet = parse_spansh_planet(&body).unwrap().unwrap();

        assert_eq!(planet.name, "Mercury");
        assert_eq!(planet.system_name, "Sol");
        assert!((planet.gravity.unwrap() - 0.375546).abs() < 0.0001);
        assert_eq!(planet.surface_temp, Some(401));
        assert_eq!(planet.pressure, Some(0.0));
        assert_eq!(planet.atmosphere.as_deref(), Some("No atmosphere"));
        assert_eq!(planet.is_mapped, None);
    }

    #[test]
    fn ignores_non_planets_and_non_landable_bodies() {
        let star = json!({"type": "Star", "name": "Sol"});
        let non_landable = json!({
            "type": "Planet",
            "name": "Jupiter",
            "system_name": "Sol",
            "is_landable": false
        });

        assert!(parse_spansh_planet(&star).unwrap().is_none());
        assert!(parse_spansh_planet(&non_landable).unwrap().is_none());
    }

    #[test]
    fn preserves_mapping_flag_if_source_adds_it() {
        let body = json!({
            "type": "Planet",
            "name": "Mercury",
            "system_name": "Sol",
            "is_landable": true,
            "is_mapped": true
        });

        assert_eq!(
            parse_spansh_planet(&body).unwrap().unwrap().is_mapped,
            Some(true)
        );
    }
}
