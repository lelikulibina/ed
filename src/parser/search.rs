use crate::parser::{core::ParserCore, planet::PlanetInfo, planet::parse_spansh_planet};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetType {
    Any,
    RockyBody,
    IcyBody,
    RockyIceWorld,
    HighMetalContentWorld,
}

impl PlanetType {
    pub fn from_str(s: &str) -> Option<Self> {
        let norm = s.trim().to_lowercase();
        if norm.is_empty() || norm == "any" {
            return Some(Self::Any);
        }

        match norm.as_str() {
            "rocky body" | "rocky" | "rock" => Some(Self::RockyBody),
            "icy body" | "icy" | "ice" => Some(Self::IcyBody),
            "rocky ice world" | "rocky-ice world" | "rocky ice" => Some(Self::RockyIceWorld),
            "high metal content world" | "high-metal content" | "high metal" => {
                Some(Self::HighMetalContentWorld)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtmosphereType {
    Any,
    Ammonia,
    Water,
    CarbonDioxide,
    SulphurDioxide,
    Nitrogen,
    WaterRich,
    MethaneRich,
    CarbonDioxideRich,
    Methane,
    Helium,
    SilicateVapour,
    NeonRich,
    ArgonRich,
    Neon,
    Argon,
    Oxygen,
}

impl AtmosphereType {
    pub fn from_str(s: &str) -> Option<Self> {
        let norm = s.trim().to_lowercase();
        if norm.is_empty() || norm == "any" {
            return Some(Self::Any);
        }

        match norm.as_str() {
            "ammonia" => Some(Self::Ammonia),
            "water" => Some(Self::Water),
            "carbon dioxide" | "carbon-dioxide" | "carbondioxide" => Some(Self::CarbonDioxide),
            "sulphur dioxide" | "sulfur dioxide" | "sulfurdioxide" => Some(Self::SulphurDioxide),
            "nitrogen" => Some(Self::Nitrogen),
            "water-rich" | "water rich" => Some(Self::WaterRich),
            "methane-rich" | "methan-rich" | "methane rich" => Some(Self::MethaneRich),
            "carbon dioxide-rich" | "carbon-dioxide rich" | "carbon dioxide rich" => {
                Some(Self::CarbonDioxideRich)
            }
            "methane" | "methan" => Some(Self::Methane),
            "helium" => Some(Self::Helium),
            "silicate vapour" | "silicate vapor" => Some(Self::SilicateVapour),
            "neon-rich" | "neon rich" => Some(Self::NeonRich),
            "argon-rich" | "argon rich" => Some(Self::ArgonRich),
            "neon" => Some(Self::Neon),
            "argon" => Some(Self::Argon),
            "oxygen" => Some(Self::Oxygen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GravityRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone)]
pub struct PlanetSearchParams {
    pub commander_position: String,
    pub planet_type: PlanetType,
    pub atmosphere: AtmosphereType,
    pub gravity: GravityRange,
    /// Number of Spansh result pages to inspect (100 bodies per page).
    pub max_pages: usize,
}

#[derive(Debug, Clone, Copy)]
struct Coordinates {
    x: f64,
    y: f64,
    z: f64,
}

fn coordinates_from_json(value: &Value) -> Option<Coordinates> {
    let coords = value.get("coords")?;
    Some(Coordinates {
        x: coords.get("x")?.as_f64()?,
        y: coords.get("y")?.as_f64()?,
        z: coords.get("z")?.as_f64()?,
    })
}

pub struct PlanetSearch {
    core: ParserCore,
    params: PlanetSearchParams,
    reference_coords: Option<Coordinates>,
    current_page: usize,
    finished: bool,
}

impl PlanetSearch {
    pub fn new(core: ParserCore, params: PlanetSearchParams) -> Self {
        Self {
            core,
            params,
            reference_coords: None,
            current_page: 0,
            finished: false,
        }
    }

    async fn load_reference_coords(&self) -> Result<Coordinates> {
        let url = format!(
            "https://www.edsm.net/api-v1/system?systemName={}&showCoordinates=1",
            urlencoding::encode(&self.params.commander_position)
        );
        let response = self.core.fetch_json(&url).await?;
        coordinates_from_json(&response).context(format!(
            "EDSM не вернул координаты опорной системы {}",
            self.params.commander_position
        ))
    }

    pub async fn next_page(&mut self) -> Result<Option<Vec<PlanetInfo>>> {
        const PAGE_SIZE: usize = 100;
        const SEARCH_RADIUS_LY: f64 = 100.0;

        if self.finished || self.current_page >= self.params.max_pages {
            return Ok(None);
        }

        if self.reference_coords.is_none() {
            self.reference_coords = Some(self.load_reference_coords().await?);
        }

        // Be polite to the public search API when pagination is required.
        if self.current_page > 0 {
            sleep(Duration::from_secs(1)).await;
        }

        let coords = self.reference_coords.expect("reference coordinates loaded");
        let payload = json!({
            "filters": {},
            "sort": [{ "distance": { "direction": "asc" } }],
            "size": PAGE_SIZE,
            "page": self.current_page,
            "reference_coords": {
                "x": coords.x,
                "y": coords.y,
                "z": coords.z
            }
        });

        let response = self
            .core
            .post_json("https://spansh.co.uk/api/bodies/search", &payload)
            .await?;
        let bodies = response
            .get("results")
            .and_then(Value::as_array)
            .context("Spansh API вернул ответ без массива results")?;

        let mut planets = Vec::new();
        for body in bodies {
            let distance = body
                .get("distance")
                .and_then(Value::as_f64)
                .context("Spansh API не вернул расстояние до тела")?;

            // Results are sorted by distance, so no later page can be inside
            // the requested radius once this boundary has been crossed.
            if distance > SEARCH_RADIUS_LY {
                self.finished = true;
                break;
            }

            if !matches_body_filters(
                body,
                self.params.planet_type,
                self.params.atmosphere,
                self.params.gravity,
            ) {
                continue;
            }

            if let Some(planet) = parse_spansh_planet(body)? {
                planets.push(planet);
            }
        }

        self.current_page += 1;
        let total_results = response.get("count").and_then(Value::as_u64);
        if bodies.is_empty()
            || bodies.len() < PAGE_SIZE
            || total_results.is_some_and(|count| self.current_page * PAGE_SIZE >= count as usize)
        {
            self.finished = true;
        }

        Ok(Some(planets))
    }

    pub fn reached_search_limit(&self) -> bool {
        self.current_page >= self.params.max_pages && !self.finished
    }
}

fn matches_body_filters(
    body: &Value,
    planet_type: PlanetType,
    atmosphere_type: AtmosphereType,
    gravity: GravityRange,
) -> bool {
    if body.get("type").and_then(Value::as_str) != Some("Planet")
        || body.get("is_landable").and_then(Value::as_bool) != Some(true)
    {
        return false;
    }

    let subtype = body
        .get("subtype")
        .or_else(|| body.get("sub_type"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .replace('-', " ");

    let matches_planet_type = match planet_type {
        PlanetType::Any => true,
        PlanetType::RockyBody => subtype.contains("rocky body"),
        PlanetType::IcyBody => subtype.contains("icy body"),
        PlanetType::RockyIceWorld => subtype.contains("rocky ice"),
        PlanetType::HighMetalContentWorld => {
            subtype.contains("high metal content") || subtype.contains("metal rich")
        }
    };
    if !matches_planet_type {
        return false;
    }

    let atmosphere = body
        .get("atmosphere")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .replace('-', " ");

    let has = |phrase: &str| atmosphere.contains(phrase);
    let matches_atmosphere = match atmosphere_type {
        AtmosphereType::Any => true,
        AtmosphereType::Ammonia => has("ammonia"),
        AtmosphereType::Water => has("water") && !has("water rich"),
        AtmosphereType::CarbonDioxide => has("carbon dioxide") && !has("carbon dioxide rich"),
        AtmosphereType::SulphurDioxide => has("sulphur dioxide") || has("sulfur dioxide"),
        AtmosphereType::Nitrogen => has("nitrogen"),
        AtmosphereType::WaterRich => has("water rich"),
        AtmosphereType::MethaneRich => has("methane rich"),
        AtmosphereType::CarbonDioxideRich => has("carbon dioxide rich"),
        AtmosphereType::Methane => has("methane") && !has("methane rich"),
        AtmosphereType::Helium => has("helium"),
        AtmosphereType::SilicateVapour => has("silicate vapour") || has("silicate vapor"),
        AtmosphereType::NeonRich => has("neon rich"),
        AtmosphereType::ArgonRich => has("argon rich"),
        AtmosphereType::Neon => has("neon") && !has("neon rich"),
        AtmosphereType::Argon => has("argon") && !has("argon rich"),
        AtmosphereType::Oxygen => has("oxygen"),
    };
    if !matches_atmosphere {
        return false;
    }

    let Some(gravity_g) = body.get("gravity").and_then(Value::as_f64) else {
        return false;
    };

    gravity_g >= gravity.min as f64 && gravity_g <= gravity.max as f64
}

#[cfg(test)]
mod api_filter_tests {
    use super::{AtmosphereType, GravityRange, PlanetType, matches_body_filters};
    use serde_json::json;

    #[test]
    fn matches_spansh_subtype_atmosphere_and_earth_gravity() {
        let body = json!({
            "type": "Planet",
            "is_landable": true,
            "subtype": "Rocky body",
            "atmosphere": "Thin ammonia atmosphere",
            "gravity": 1.0
        });

        assert!(matches_body_filters(
            &body,
            PlanetType::RockyBody,
            AtmosphereType::Ammonia,
            GravityRange { min: 0.9, max: 1.1 }
        ));
    }

    #[test]
    fn rejects_wrong_atmosphere_and_gravity() {
        let body = json!({
            "type": "Planet",
            "is_landable": true,
            "subtype": "Rocky body",
            "atmosphere": "Thin ammonia atmosphere",
            "gravity": 2.0
        });

        assert!(!matches_body_filters(
            &body,
            PlanetType::RockyBody,
            AtmosphereType::Water,
            GravityRange { min: 0.9, max: 2.1 }
        ));
        assert!(!matches_body_filters(
            &body,
            PlanetType::RockyBody,
            AtmosphereType::Ammonia,
            GravityRange { min: 0.9, max: 1.1 }
        ));
    }
}
