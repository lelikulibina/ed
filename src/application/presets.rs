use crate::domain::criteria::NumericRange;

#[derive(Debug, Clone, Copy)]
pub struct SearchPreset {
    pub number: u8,
    pub name: &'static str,
    pub surface_types: &'static [&'static str],
    pub atmosphere: &'static str,
    pub temperature: NumericRange,
    pub pressure: NumericRange,
    pub expensive_species: u8,
    pub potential_value_cr: u64,
}

const fn range(min: f64, max: f64) -> NumericRange {
    NumericRange { min, max }
}

pub const PRESETS: [SearchPreset; 7] = [
    SearchPreset {
        number: 1,
        name: "CO₂ / Rocky · 190–195 K",
        surface_types: &["Rocky body"],
        atmosphere: "Carbon dioxide",
        temperature: range(190.0, 195.0),
        pressure: range(0.042, 0.237),
        expensive_species: 9,
        potential_value_cr: 363_000_000,
    },
    SearchPreset {
        number: 2,
        name: "CO₂ / Rocky · 180–190 K",
        surface_types: &["Rocky body"],
        atmosphere: "Carbon dioxide",
        temperature: range(180.0, 190.0),
        pressure: range(0.042, 0.275),
        expensive_species: 7,
        potential_value_cr: 166_000_000,
    },
    SearchPreset {
        number: 3,
        name: "Water / Rocky + High metal",
        surface_types: &["Rocky body", "High metal content world"],
        atmosphere: "Water",
        temperature: range(395.0, 450.0),
        pressure: range(0.040, 0.063),
        expensive_species: 6,
        potential_value_cr: 279_000_000,
    },
    SearchPreset {
        number: 4,
        name: "Ammonia / Rocky · 165–175 K",
        surface_types: &["Rocky body"],
        atmosphere: "Ammonia",
        temperature: range(165.0, 175.0),
        pressure: range(0.040, 0.340),
        expensive_species: 4,
        potential_value_cr: 94_000_000,
    },
    SearchPreset {
        number: 5,
        name: "Argon / Icy · 50–150 K",
        surface_types: &["Icy body"],
        atmosphere: "Argon",
        temperature: range(50.0, 150.0),
        pressure: range(0.167, 0.272),
        expensive_species: 3,
        potential_value_cr: 65_000_000,
    },
    SearchPreset {
        number: 6,
        name: "SO₂ / Rocky · 132–195 K",
        surface_types: &["Rocky body"],
        atmosphere: "Sulphur dioxide",
        temperature: range(132.0, 195.0),
        pressure: range(0.044, 0.275),
        expensive_species: 2,
        potential_value_cr: 141_000_000,
    },
    SearchPreset {
        number: 7,
        name: "Oxygen / Icy · 145–270 K",
        surface_types: &["Icy body"],
        atmosphere: "Oxygen",
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
            assert_eq!(preset.surface_types.join("|"), row.1);
            assert_eq!(preset.atmosphere, row.2);
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
        }
    }
}
