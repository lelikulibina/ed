use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceType {
    Any,
    RockyBody,
    IcyBody,
    RockyIceWorld,
    HighMetalContentWorld,
}

impl SurfaceType {
    pub const ALL: [Self; 5] = [
        Self::Any,
        Self::RockyBody,
        Self::IcyBody,
        Self::RockyIceWorld,
        Self::HighMetalContentWorld,
    ];

    pub const fn spansh_value(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::RockyBody => "Rocky body",
            Self::IcyBody => "Icy body",
            Self::RockyIceWorld => "Rocky ice world",
            Self::HighMetalContentWorld => "High metal content world",
        }
    }

    pub const fn label_ru(self) -> &'static str {
        match self {
            Self::Any => "Любой тип",
            Self::RockyBody => "Каменистая",
            Self::IcyBody => "Ледяная",
            Self::RockyIceWorld => "Каменисто-ледяная",
            Self::HighMetalContentWorld => "С высоким содержанием металлов",
        }
    }
}

impl FromStr for SurfaceType {
    type Err = CatalogParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "any" => Ok(Self::Any),
            "rocky body" | "rocky" | "rock" => Ok(Self::RockyBody),
            "icy body" | "icy" | "ice" => Ok(Self::IcyBody),
            "rocky ice world" | "rocky-ice world" | "rocky ice" => Ok(Self::RockyIceWorld),
            "high metal content world" | "high-metal content" | "high metal" => {
                Ok(Self::HighMetalContentWorld)
            }
            _ => Err(CatalogParseError::SurfaceType(value.to_owned())),
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
    pub const ALL: [Self; 17] = [
        Self::Any,
        Self::Ammonia,
        Self::Water,
        Self::CarbonDioxide,
        Self::SulphurDioxide,
        Self::Nitrogen,
        Self::WaterRich,
        Self::MethaneRich,
        Self::CarbonDioxideRich,
        Self::Methane,
        Self::Helium,
        Self::SilicateVapour,
        Self::NeonRich,
        Self::ArgonRich,
        Self::Neon,
        Self::Argon,
        Self::Oxygen,
    ];

    pub const fn spansh_value(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Ammonia => "Ammonia",
            Self::Water => "Water",
            Self::CarbonDioxide => "Carbon dioxide",
            Self::SulphurDioxide => "Sulphur dioxide",
            Self::Nitrogen => "Nitrogen",
            Self::WaterRich => "Water-rich",
            Self::MethaneRich => "Methane-rich",
            Self::CarbonDioxideRich => "Carbon dioxide-rich",
            Self::Methane => "Methane",
            Self::Helium => "Helium",
            Self::SilicateVapour => "Silicate vapour",
            Self::NeonRich => "Neon-rich",
            Self::ArgonRich => "Argon-rich",
            Self::Neon => "Neon",
            Self::Argon => "Argon",
            Self::Oxygen => "Oxygen",
        }
    }

    pub const fn label_ru(self) -> &'static str {
        match self {
            Self::Any => "Любая атмосфера",
            Self::Ammonia => "Аммиачная",
            Self::Water => "Водная",
            Self::CarbonDioxide => "Углекислотная",
            Self::SulphurDioxide => "Сернистая",
            Self::Nitrogen => "Азотная",
            Self::WaterRich => "Богатая водой",
            Self::MethaneRich => "Богатая метаном",
            Self::CarbonDioxideRich => "Богатая углекислым газом",
            Self::Methane => "Метановая",
            Self::Helium => "Гелиевая",
            Self::SilicateVapour => "Силикатный пар",
            Self::NeonRich => "Богатая неоном",
            Self::ArgonRich => "Богатая аргоном",
            Self::Neon => "Неоновая",
            Self::Argon => "Аргоновая",
            Self::Oxygen => "Кислородная",
        }
    }
}

impl FromStr for AtmosphereType {
    type Err = CatalogParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "any" => Ok(Self::Any),
            "ammonia" => Ok(Self::Ammonia),
            "water" => Ok(Self::Water),
            "carbon dioxide" | "carbon-dioxide" | "carbondioxide" => Ok(Self::CarbonDioxide),
            "sulphur dioxide" | "sulphur-dioxide" | "sulfur dioxide" | "sulfur-dioxide"
            | "sulfurdioxide" => Ok(Self::SulphurDioxide),
            "nitrogen" => Ok(Self::Nitrogen),
            "water-rich" | "water rich" => Ok(Self::WaterRich),
            "methane-rich" | "methan-rich" | "methane rich" => Ok(Self::MethaneRich),
            "carbon dioxide-rich" | "carbon-dioxide rich" | "carbon dioxide rich" => {
                Ok(Self::CarbonDioxideRich)
            }
            "methane" | "methan" => Ok(Self::Methane),
            "helium" => Ok(Self::Helium),
            "silicate vapour" | "silicate vapor" => Ok(Self::SilicateVapour),
            "neon-rich" | "neon rich" => Ok(Self::NeonRich),
            "argon-rich" | "argon rich" => Ok(Self::ArgonRich),
            "neon" => Ok(Self::Neon),
            "argon" => Ok(Self::Argon),
            "oxygen" => Ok(Self::Oxygen),
            _ => Err(CatalogParseError::AtmosphereType(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogParseError {
    #[error("неизвестный тип поверхности планеты: {0}")]
    SurfaceType(String),
    #[error("неизвестный тип атмосферы: {0}")]
    AtmosphereType(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_every_legacy_surface_type_value() {
        let cases = [
            ("any", SurfaceType::Any),
            ("rocky body", SurfaceType::RockyBody),
            ("rocky", SurfaceType::RockyBody),
            ("icy body", SurfaceType::IcyBody),
            ("icy", SurfaceType::IcyBody),
            ("rocky ice world", SurfaceType::RockyIceWorld),
            (
                "high metal content world",
                SurfaceType::HighMetalContentWorld,
            ),
        ];

        for (legacy, expected) in cases {
            assert_eq!(legacy.parse::<SurfaceType>().unwrap(), expected, "{legacy}");
        }
    }

    #[test]
    fn accepts_every_legacy_atmosphere_type_value() {
        let cases = [
            ("any", AtmosphereType::Any),
            ("ammonia", AtmosphereType::Ammonia),
            ("water", AtmosphereType::Water),
            ("carbon-dioxide", AtmosphereType::CarbonDioxide),
            ("sulphur dioxide", AtmosphereType::SulphurDioxide),
            ("sulphur-dioxide", AtmosphereType::SulphurDioxide),
            ("nitrogen", AtmosphereType::Nitrogen),
            ("water-rich", AtmosphereType::WaterRich),
            ("methane-rich", AtmosphereType::MethaneRich),
            ("carbon dioxide-rich", AtmosphereType::CarbonDioxideRich),
            ("methane", AtmosphereType::Methane),
            ("helium", AtmosphereType::Helium),
            ("silicate vapour", AtmosphereType::SilicateVapour),
            ("neon-rich", AtmosphereType::NeonRich),
            ("argon-rich", AtmosphereType::ArgonRich),
            ("neon", AtmosphereType::Neon),
            ("argon", AtmosphereType::Argon),
            ("oxygen", AtmosphereType::Oxygen),
        ];

        for (legacy, expected) in cases {
            assert_eq!(
                legacy.parse::<AtmosphereType>().unwrap(),
                expected,
                "{legacy}"
            );
        }
    }

    #[test]
    fn catalog_values_round_trip_and_have_russian_labels() {
        for surface in SurfaceType::ALL {
            assert!(!surface.label_ru().is_empty());
            assert_eq!(
                surface.spansh_value().parse::<SurfaceType>().unwrap(),
                surface
            );
        }
        for atmosphere in AtmosphereType::ALL {
            assert!(!atmosphere.label_ru().is_empty());
            assert_eq!(
                atmosphere.spansh_value().parse::<AtmosphereType>().unwrap(),
                atmosphere
            );
        }
    }
}
