use anyhow::{Context, Result, bail};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PlanetInfo {
    pub name: String,
    /// None means the API does not provide a mapping status for this body.
    pub is_mapped: Option<bool>,
    pub surface_temp: Option<i32>,
    /// Gravity in Earth gravities (g).
    pub gravity: Option<f32>,
    pub atmosphere: Option<String>,
    /// Surface pressure in atmospheres.
    pub pressure: Option<f32>,
}

/// Converts one EDSM API body into the values used by the CLI.
///
/// EDSM reports gravity in m/s² and pressure in pascals. The CLI has
/// historically accepted/displayed Earth gravities and atmospheres.
pub fn parse_api_planet(body: &Value) -> Result<Option<PlanetInfo>> {
    if body.get("type").and_then(Value::as_str) != Some("Planet")
        || body.get("isLandable").and_then(Value::as_bool) != Some(true)
    {
        return Ok(None);
    }

    let name = body
        .get("name")
        .and_then(Value::as_str)
        .context("EDSM API returned a landable planet without a name")?
        .to_string();

    let gravity = body
        .get("gravity")
        .and_then(Value::as_f64)
        .map(|value| (value / 9.80665) as f32);

    let pressure = body
        .get("surfacePressure")
        .and_then(Value::as_f64)
        .map(|value| (value / 101_325.0) as f32);

    let surface_temp = body
        .get("surfaceTemperature")
        .and_then(Value::as_f64)
        .map(|value| value as i32);

    let atmosphere = body
        .get("atmosphereType")
        .and_then(Value::as_str)
        .map(str::to_string);

    // The documented EDSM bodies API exposes discovery metadata, but does
    // not document a mapped flag. Preserve it only if the API supplies one.
    let is_mapped = body.get("isMapped").and_then(Value::as_bool);

    if gravity.is_none() || surface_temp.is_none() {
        bail!("EDSM API returned incomplete physical data for planet {name}");
    }

    Ok(Some(PlanetInfo {
        name,
        is_mapped,
        surface_temp,
        gravity,
        atmosphere,
        pressure,
    }))
}

#[cfg(test)]
mod tests {
    use super::parse_api_planet;
    use serde_json::json;

    #[test]
    fn converts_edsm_api_units_to_cli_units() {
        let body = json!({
            "type": "Planet",
            "name": "Example A 1",
            "subType": "Rocky body",
            "isLandable": true,
            "gravity": 9.80665,
            "surfaceTemperature": 245,
            "surfacePressure": 101325,
            "atmosphereType": "Thin ammonia atmosphere"
        });

        let planet = parse_api_planet(&body).unwrap().unwrap();

        assert_eq!(planet.name, "Example A 1");
        assert!((planet.gravity.unwrap() - 1.0).abs() < 0.0001);
        assert_eq!(planet.surface_temp, Some(245));
        assert!((planet.pressure.unwrap() - 1.0).abs() < 0.0001);
        assert_eq!(planet.is_mapped, None);
    }

    #[test]
    fn ignores_non_landable_bodies_and_stars() {
        let star = json!({"type": "Star", "name": "Example"});
        let non_landable = json!({
            "type": "Planet",
            "name": "Example 1",
            "isLandable": false
        });

        assert!(parse_api_planet(&star).unwrap().is_none());
        assert!(parse_api_planet(&non_landable).unwrap().is_none());
    }

    #[test]
    fn preserves_mapping_flag_when_api_supplies_it() {
        let body = json!({
            "type": "Planet",
            "name": "Example 1",
            "isLandable": true,
            "gravity": 9.80665,
            "surfaceTemperature": 250,
            "isMapped": true
        });

        assert_eq!(parse_api_planet(&body).unwrap().unwrap().is_mapped, Some(true));
    }
}
