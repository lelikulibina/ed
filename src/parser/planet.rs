use crate::parser::core::ParserCore;
use anyhow::Result;
use scraper::Selector;
use anyhow::Result;
use scraper::Selector;
use crate::parser::core::ParserCore;

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
    let row_sel = Selector::parse(".row").unwrap();
    let col4_sel = Selector::parse(".col-lg-4").unwrap();
    let col8_sel = Selector::parse(".col-lg-8").unwrap();

    let mut info = PlanetInfo {
        name: name.to_string(),
        is_mapped: false,
        surface_temp: None,
        gravity: None,
        atmosphere: None,
        pressure: None,
    };

    for row in doc.select(&row_sel) {
        let label = row.select(&col4_sel).next();
        let value = row.select(&col8_sel).next();

        if let (Some(l), Some(v)) = (label, value) {
            let key = l.text().collect::<String>().trim().to_string();
            let val = v.text().collect::<String>().trim().to_string();

            match key.as_str() {
                "First mapped for EDSM by:" => info.is_mapped = true,
                "Surface temperature:" => info.surface_temp = parse_int(&val),
                "Surface pressure:" => info.pressure = parse_float(&val),
                "Gravity:" => info.gravity = parse_float(&val),
                "Atmosphere:" => info.atmosphere = Some(val),
                _ => {}
            }
        }
    }

    Ok(info)
}
