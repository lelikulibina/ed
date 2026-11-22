use crate::domain::{
    catalog::{AtmosphereType, SurfaceType},
    criteria::{CriteriaError, Filter, NumericRange, SearchCriteria},
};

pub const PRESET_GRAVITY_RANGE: NumericRange = NumericRange {
    min: 0.1,
    max: 1.0,
};

#[derive(Debug, Clone, Copy)]
pub struct SearchPreset {
    pub number: u8,
    pub name: &'static str,
    pub surface_types: &'static [SurfaceType],
    pub atmosphere: AtmosphereType,
    pub temperature: NumericRange,
    pub pressure: NumericRange,
    pub expensive_species: u8,
    pub potential_value_cr: u64,
}

const fn range(min: f64, max: f64) -> NumericRange {
    NumericRange { min, max }
}

impl SearchPreset {
    pub fn to_criteria(
        &self,
        count: usize,
        exclude_with_bio_signals: bool,
    ) -> Result<SearchCriteria, CriteriaError> {
        let surface = if self.surface_types.contains(&SurfaceType::Any) {
            Filter::Any
        } else {
            Filter::one_of(self.surface_types.to_vec())
        };
        let atmosphere = if self.atmosphere == AtmosphereType::Any {
            Filter::Any
        } else {
            Filter::one_of(vec![self.atmosphere])
        };

        SearchCriteria::new(
            surface,
            atmosphere,
            self.temperature,
            PRESET_GRAVITY_RANGE,
            self.pressure,
            count,
        )
        .map(|criteria| criteria.with_bio_signal_exclusion(exclude_with_bio_signals))
    }
}

pub const PRESETS: [SearchPreset; 7] = [
    SearchPreset {
        number: 1,
        name: "CO₂ / Rocky · 190–195 K",
        surface_types: &[SurfaceType::RockyBody],
        atmosphere: AtmosphereType::CarbonDioxide,
        temperature: range(190.0, 195.0),
        pressure: range(0.042, 0.237),
        expensive_species: 9,
        potential_value_cr: 363_000_000,
    },
    SearchPreset {
        number: 2,
        name: "CO₂ / Rocky · 180–190 K",
        surface_types: &[SurfaceType::RockyBody],
        atmosphere: AtmosphereType::CarbonDioxide,
        temperature: range(180.0, 190.0),
        pressure: range(0.042, 0.275),
        expensive_species: 7,
        potential_value_cr: 166_000_000,
    },
    SearchPreset {
        number: 3,
        name: "Water / Rocky + High metal",
        surface_types: &[SurfaceType::RockyBody, SurfaceType::HighMetalContentWorld],
        atmosphere: AtmosphereType::Water,
        temperature: range(395.0, 450.0),
        pressure: range(0.040, 0.063),
        expensive_species: 6,
        potential_value_cr: 279_000_000,
    },
    SearchPreset {
        number: 4,
        name: "Ammonia / Rocky · 165–175 K",
        surface_types: &[SurfaceType::RockyBody],
        atmosphere: AtmosphereType::Ammonia,
        temperature: range(165.0, 175.0),
        pressure: range(0.040, 0.340),
        expensive_species: 4,
        potential_value_cr: 94_000_000,
    },
    SearchPreset {
        number: 5,
        name: "Argon / Icy · 50–150 K",
        surface_types: &[SurfaceType::IcyBody],
        atmosphere: AtmosphereType::Argon,
        temperature: range(50.0, 150.0),
        pressure: range(0.167, 0.272),
        expensive_species: 3,
        potential_value_cr: 65_000_000,
    },
    SearchPreset {
        number: 6,
        name: "SO₂ / Rocky · 132–195 K",
        surface_types: &[SurfaceType::RockyBody],
        atmosphere: AtmosphereType::SulphurDioxide,
        temperature: range(132.0, 195.0),
        pressure: range(0.044, 0.275),
        expensive_species: 2,
        potential_value_cr: 141_000_000,
    },
    SearchPreset {
        number: 7,
        name: "Oxygen / Icy · 145–270 K",
        surface_types: &[SurfaceType::IcyBody],
        atmosphere: AtmosphereType::Oxygen,
        temperature: range(145.0, 270.0),
        pressure: range(0.238, 0.275),
        expensive_species: 2,
        potential_value_cr: 111_000_000,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_the_seven_user_defined_profiles_exactly() {
        let expected = [
            (
                1,
                "Rocky body",
                "Carbon dioxide",
                190.0,
                195.0,
                0.042,
                0.237,
                9,
                363_000_000_u64,
            ),
            (
                2,
                "Rocky body",
                "Carbon dioxide",
                180.0,
                190.0,
                0.042,
                0.275,
                7,
                166_000_000_u64,
            ),
            (
                3,
                "Rocky body|High metal content world",
                "Water",
                395.0,
                450.0,
                0.040,
                0.063,
                6,
                279_000_000_u64,
            ),
            (
                4,
                "Rocky body",
                "Ammonia",
                165.0,
                175.0,
                0.040,
                0.340,
                4,
                94_000_000_u64,
            ),
            (
                5,
                "Icy body",
                "Argon",
                50.0,
                150.0,
                0.167,
                0.272,
                3,
                65_000_000_u64,
            ),
            (
                6,
                "Rocky body",
                "Sulphur dioxide",
                132.0,
                195.0,
                0.044,
                0.275,
                2,
                141_000_000_u64,
            ),
            (
                7,
                "Icy body",
                "Oxygen",
                145.0,
                270.0,
                0.238,
                0.275,
                2,
                111_000_000_u64,
            ),
        ];

        assert_eq!(PRESETS.len(), expected.len());
        for (preset, row) in PRESETS.iter().zip(expected) {
            assert_eq!(preset.number, row.0);
            assert_eq!(
                preset
                    .surface_types
                    .iter()
                    .map(|surface| surface.spansh_value())
                    .collect::<Vec<_>>()
                    .join("|"),
                row.1
            );
            assert_eq!(preset.atmosphere.spansh_value(), row.2);
            assert_eq!(
                preset.temperature,
                NumericRange {
                    min: row.3,
                    max: row.4
                }
            );
            assert_eq!(
                preset.pressure,
                NumericRange {
                    min: row.5,
                    max: row.6
                }
            );
            assert_eq!(preset.expensive_species, row.7);
            assert_eq!(preset.potential_value_cr, row.8);

            let criteria = preset.to_criteria(5, true).unwrap();
            assert_eq!(
                criteria.surface,
                if preset.surface_types.contains(&SurfaceType::Any) {
                    Filter::Any
                } else {
                    Filter::one_of(preset.surface_types.to_vec())
                }
            );
            assert_eq!(
                criteria.atmosphere,
                if preset.atmosphere == AtmosphereType::Any {
                    Filter::Any
                } else {
                    Filter::one_of(vec![preset.atmosphere])
                }
            );
            assert_eq!(criteria.temperature, preset.temperature);
            assert_eq!(criteria.pressure, preset.pressure);
            assert_eq!(criteria.gravity, NumericRange { min: 0.1, max: 1.0 });
            assert_eq!(criteria.count, 5);
            assert!(criteria.exclude_with_bio_signals);
        }
    }
}
