use crate::parser::core::ParserCore;
use anyhow::Result;
use scraper::Selector;

#[derive(Debug)]
pub struct BodyLink {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetType {
    Any,
    RockyBody,             // 11
    IcyBody,               // 21
    RockyIceWorld,         // 12
    HighMetalContentWorld, // 2
}

impl PlanetType {
    pub fn to_code(&self) -> u16 {
        match self {
            PlanetType::Any => 255,
            PlanetType::RockyBody => 11,
            PlanetType::IcyBody => 21,
            PlanetType::RockyIceWorld => 12,
            PlanetType::HighMetalContentWorld => 2,
        }
    }

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
    Ammonia,           // 3
    Water,             // 4
    CarbonDioxide,     // 5
    SulphurDioxide,    // 6
    Nitrogen,          // 7
    WaterRich,         // 8
    MethaneRich,       // 9
    CarbonDioxideRich, // 11
    Methane,           // 12
    Helium,            // 13
    SilicateVapour,    // 14
    NeonRich,          // 16
    ArgonRich,         // 17
    Neon,              // 18
    Argon,             // 19
    Oxygen,            // 20
}

impl AtmosphereType {
    pub fn to_code(&self) -> u8 {
        match self {
            AtmosphereType::Any => 255, // использовать как "без фильтра"
            AtmosphereType::Ammonia => 3,
            AtmosphereType::Water => 4,
            AtmosphereType::CarbonDioxide => 5,
            AtmosphereType::SulphurDioxide => 6,
            AtmosphereType::Nitrogen => 7,
            AtmosphereType::WaterRich => 8,
            AtmosphereType::MethaneRich => 9,
            AtmosphereType::CarbonDioxideRich => 11,
            AtmosphereType::Methane => 12,
            AtmosphereType::Helium => 13,
            AtmosphereType::SilicateVapour => 14,
            AtmosphereType::NeonRich => 16,
            AtmosphereType::ArgonRich => 17,
            AtmosphereType::Neon => 18,
            AtmosphereType::Argon => 19,
            AtmosphereType::Oxygen => 20,
        }
    }

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
    pub max_pages: usize,
}

impl PlanetSearchParams {
    fn base_url(&self) -> String {
        let sector = self
            .commander_position
            .split_whitespace()
            .next()
            .unwrap_or(&self.commander_position);

        format!(
            "https://www.edsm.net/search/bodies/index/\
             atmosphere/{}/cmdrPosition/{}/filterBy/onlyScanned/\
             gravity/{:.1}%2C{:.1}/group/2/isLandable/1/name/{}/\
             solidComposition/2/solidComposition/3/\
             sortBy/distanceToArrivalASC/type/{}",
            self.atmosphere.to_code(),
            urlencoding::encode(&self.commander_position),
            self.gravity.min,
            self.gravity.max,
            urlencoding::encode(sector),
            self.planet_type.to_code()
        )
    }

    #[allow(dead_code)]
    fn extract_sector(full: &str) -> &str {
        let mut pos = 0;
        let mut iter = full.split_whitespace().peekable();

        while let Some(word) = iter.next() {
            if Self::looks_like_code(word) {
                return &full[..pos.max(1) - 1];
            }

            pos += word.len() + 1;
        }

        full
    }

    fn looks_like_code(w: &str) -> bool {
        let mut parts = w.split('-');

        let left = parts.next().unwrap_or("");
        let right = parts.next().unwrap_or("");

        // формат: <буквы>-<буквы/цифры>
        !right.is_empty()
            && left.chars().all(|c| c.is_ascii_alphabetic())
            && right.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

async fn parse_table_page(core: &ParserCore, url: &str, page: usize) -> Result<Vec<BodyLink>> {
    let document = core.fetch_html(&format!("{}/p/{}", url, page)).await?;
    let selector = Selector::parse("td a strong").unwrap();

    Ok(document
        .select(&selector)
        .filter_map(|el| {
            let parent = el.parent()?.value().as_element()?;
            let href = parent.attr("href")?;
            (href.contains("/system/bodies/")).then(|| BodyLink {
                name: el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                url: format!("https://www.edsm.net{}", href),
            })
        })
        .collect())
}

pub struct PlanetSearch {
    core: ParserCore,
    base_url: String,
    current_page: usize,
    finished: bool,
    max_pages: usize,
}

impl PlanetSearch {
    pub fn new(core: ParserCore, params: PlanetSearchParams) -> Self {
        Self {
            base_url: params.base_url(),
            core,
            current_page: 0,
            finished: false,
            max_pages: params.max_pages,
        }
    }

    pub async fn next_page(&mut self) -> Result<Option<Vec<BodyLink>>> {
        if self.finished || self.current_page >= self.max_pages {
            self.finished = true;
            return Ok(None);
        }

        self.current_page += 1;
        let links = parse_table_page(&self.core, &self.base_url, self.current_page).await?;

        if links.is_empty() {
            self.finished = true;
            Ok(None)
        } else {
            Ok(Some(links))
        }
    }
}
