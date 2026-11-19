use crate::parser::{core::ParserCore, planet::parse_api_planet, planet::PlanetInfo};
use anyhow::{Context, Result, bail};
use futures::stream::{self, StreamExt};
use serde_json::Value;
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
    /// Number of batches of nearby systems to inspect. Each batch contains
    /// up to ten systems, ordered by distance from the reference system.
    pub max_pages: usize,
}

#[derive(Debug, Clone)]
struct SystemRef {
    name: String,
    id: u64,
    distance: f64,
}

pub struct PlanetSearch {
    core: ParserCore,
    params: PlanetSearchParams,
    systems: Option<Vec<SystemRef>>,
    current_system: usize,
    current_page: usize,
    finished: bool,
}

impl PlanetSearch {
    pub fn new(core: ParserCore, params: PlanetSearchParams) -> Self {
        Self {
            core,
            params,
            systems: None,
            current_system: 0,
            current_page: 0,
            finished: false,
        }
    }

    async fn load_systems(&self) -> Result<Vec<SystemRef>> {
        let url = format!(
            "https://www.edsm.net/api-v1/sphere-systems?systemName={}&radius=100&showId=1",
            urlencoding::encode(&self.params.commander_position)
        );
        let response = self.core.fetch_json(&url).await?;
        let entries = response
            .as_array()
            .context("EDSM sphere-systems API вернул ответ не в виде массива")?;

        let mut systems = Vec::with_capacity(entries.len());
        for entry in entries {
            let Some(name) = entry.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(id) = entry.get("id").and_then(Value::as_u64) else {
                bail!("EDSM не вернул ID для системы {name}; невозможно однозначно запросить её тела");
            };
            systems.push(SystemRef {
                name: name.to_string(),
                id,
                distance: entry
                    .get("distance")
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::MAX),
            });
        }

        systems.sort_by(|left, right| {
            left.distance
                .partial_cmp(&right.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if systems.is_empty() {
            bail!(
                "EDSM API не нашёл систем в радиусе 100 св. лет от {}",
                self.params.commander_position
            );
        }

        Ok(systems)
    }

    async fn load_system_bodies(&self, system: SystemRef) -> Result<Vec<PlanetInfo>> {
        let url = format!(
            "https://www.edsm.net/api-system-v1/bodies?systemName={}&systemId={}",
            urlencoding::encode(&system.name),
            system.id
        );
        let response = self.core.fetch_json(&url).await?;
        let bodies = response
            .get("bodies")
            .and_then(Value::as_array)
            .with_context(|| format!("EDSM API не вернул список тел системы {}", system.name))?;

        let mut planets = Vec::new();
        for body in bodies {
            if !matches_body_filters(
                body,
                self.params.planet_type,
                self.params.atmosphere,
                self.params.gravity,
            ) {
                continue;
            }

            if let Some(planet) = parse_api_planet(body)? {
                planets.push(planet);
            }
        }

        // EDSM documents a limit of roughly 700 system-body requests per
        // minute. This delay, combined with bounded concurrency below,
        // keeps this client comfortably below that rate.
        sleep(Duration::from_millis(500)).await;
        Ok(planets)
    }

    pub async fn next_page(&mut self) -> Result<Option<Vec<PlanetInfo>>> {
        if self.finished || self.current_page >= self.params.max_pages {
            self.finished = true;
            return Ok(None);
        }

        if self.systems.is_none() {
            self.systems = Some(self.load_systems().await?);
        }

        let systems = self.systems.as_ref().expect("systems loaded");
        if self.current_system >= systems.len() {
            self.finished = true;
            return Ok(None);
        }

        let end = (self.current_system + 10).min(systems.len());
        let batch = systems[self.current_system..end].to_vec();
        self.current_system = end;
        self.current_page += 1;

        let results = stream::iter(batch)
            .map(|system| async move { self.load_system_bodies(system).await })
            .buffer_unordered(4)
            .collect::<Vec<_>>()
            .await;

        let mut planets = Vec::new();
        for result in results {
            planets.extend(result?);
        }

        Ok(Some(planets))
    }
}

fn matches_body_filters(
    body: &Value,
    planet_type: PlanetType,
    atmosphere_type: AtmosphereType,
    gravity: GravityRange,
) -> bool {
    if body.get("type").and_then(Value::as_str) != Some("Planet")
        || body.get("isLandable").and_then(Value::as_bool) != Some(true)
    {
        return false;
    }

    let subtype = body
        .get("subType")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .replace('-', " ");

    let matches_planet_type = match planet_type {
        PlanetType::Any => true,
        PlanetType::RockyBody => subtype.contains("rocky body"),
        PlanetType::IcyBody => subtype.contains("icy body"),
        PlanetType::RockyIceWorld => subtype.contains("rocky ice"),
        PlanetType::HighMetalContentWorld => subtype.contains("high metal content"),
    };
    if !matches_planet_type {
        return false;
    }

    let atmosphere = body
        .get("atmosphereType")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .replace('-', " ");

    let has = |phrase: &str| atmosphere.contains(phrase);
    let matches_atmosphere = match atmosphere_type {
        AtmosphereType::Any => true,
        AtmosphereType::Ammonia => has("ammonia"),
        AtmosphereType::Water => has("water") && !has("water rich"),
        AtmosphereType::CarbonDioxide => {
            has("carbon dioxide") && !has("carbon dioxide rich")
        }
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

    let Some(gravity_g) = body
        .get("gravity")
        .and_then(Value::as_f64)
        .map(|value| value / 9.80665)
    else {
        return false;
    };

    gravity_g >= gravity.min as f64 && gravity_g <= gravity.max as f64
}

#[cfg(test)]
mod api_filter_tests {
    use super::{matches_body_filters, AtmosphereType, GravityRange, PlanetType};
    use serde_json::json;

    #[test]
    fn matches_edsm_subtype_atmosphere_and_earth_gravity() {
        let body = json!({
            "type": "Planet",
            "isLandable": true,
            "subType": "Rocky body",
            "atmosphereType": "Thin ammonia atmosphere",
            "gravity": 9.80665
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
            "isLandable": true,
            "subType": "Rocky body",
            "atmosphereType": "Thin ammonia atmosphere",
            "gravity": 19.6133
        });

        assert!(!matches_body_filters(
            &body,
            PlanetType::RockyBody,
            AtmosphereType::Water,
            GravityRange { min: 0.9, max: 1.1 }
        ));
    }
}
