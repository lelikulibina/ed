use crate::parser::core::ParserCore;
use anyhow::{Result, bail};
use scraper::Selector;

#[derive(Debug, Clone)]
pub struct PlanetInfo {
    pub name: String,
    pub is_mapped: bool,
    pub surface_temp: Option<i32>,
    pub gravity: Option<f32>,
    pub atmosphere: Option<String>,
    pub pressure: Option<f32>,
}

fn parse_float(value: &str) -> Option<f32> {
    value
        .split_whitespace()
        .next()?
        .replace(',', ".")
        .parse::<f32>()
        .ok()
}

fn parse_int(value: &str) -> Option<i32> {
    value
        .split_whitespace()
        .next()?
        .replace(',', ".")
        .parse::<i32>()
        .ok()
}

pub async fn parse_planet(core: &ParserCore, url: &str, name: &str) -> Result<PlanetInfo> {
    let doc = core.fetch_html(url).await?;
    let row_sel = Selector::parse(".row").expect("valid static selector");
    let col4_sel = Selector::parse(".col-lg-4").expect("valid static selector");
    let col8_sel = Selector::parse(".col-lg-8").expect("valid static selector");

    let mut info = PlanetInfo {
        name: name.to_string(),
        is_mapped: false,
        surface_temp: None,
        gravity: None,
        atmosphere: None,
        pressure: None,
    };
    let mut recognized_fields = 0usize;

    for row in doc.select(&row_sel) {
        let label = row.select(&col4_sel).next();
        let value = row.select(&col8_sel).next();

        if let (Some(label), Some(value)) = (label, value) {
            let key = label.text().collect::<String>().trim().to_string();
            let val = value.text().collect::<String>().trim().to_string();

            match key.as_str() {
                "First mapped for EDSM by:" => {
                    info.is_mapped = true;
                    recognized_fields += 1;
                }
                "Surface temperature:" => {
                    info.surface_temp = parse_int(&val);
                    recognized_fields += 1;
                }
                "Surface pressure:" => {
                    info.pressure = parse_float(&val);
                    recognized_fields += 1;
                }
                "Gravity:" => {
                    info.gravity = parse_float(&val);
                    recognized_fields += 1;
                }
                "Atmosphere:" => {
                    info.atmosphere = Some(val);
                    recognized_fields += 1;
                }
                _ => {}
            }
        }
    }

    if recognized_fields == 0 {
        bail!(
            "Страница планеты {name} не содержит ожидаемых полей EDSM; возможно, изменилась HTML-разметка или язык страницы ({url})"
        );
    }

    Ok(info)
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
            "isMapped": true
        });

        assert_eq!(parse_api_planet(&body).unwrap().unwrap().is_mapped, Some(true));
    }
}
