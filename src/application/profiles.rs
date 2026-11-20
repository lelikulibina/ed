use crate::domain::criteria::{NumericRange, SearchCriteria, VolcanismFilter};

#[derive(Debug, Clone, Copy)]
pub struct SearchProfile {
    pub number: usize,
    pub name: &'static str,
    pub planet_types: &'static [&'static str],
    pub atmosphere: &'static str,
    pub temperature: (f64, f64),
    pub pressure: (f64, f64),
    pub gravity: (f64, f64),
    pub volcanism: VolcanismFilter,
    pub max_expensive_species: u8,
    pub potential_max_credits: u64,
}

impl SearchProfile {
    pub fn criteria(
        &self,
        count: usize,
    ) -> Result<SearchCriteria, crate::domain::criteria::CriteriaError> {
        SearchCriteria::from_types(
            self.planet_types,
            self.atmosphere,
            NumericRange {
                min: self.temperature.0,
                max: self.temperature.1,
            },
            NumericRange {
                min: self.gravity.0,
                max: self.gravity.1,
            },
            NumericRange {
                min: self.pressure.0,
                max: self.pressure.1,
            },
            count,
        )
        .map(|criteria| criteria.with_volcanism_filter(self.volcanism))
    }
}

pub const SEARCH_PROFILES: [SearchProfile; 7] = [
    SearchProfile {
        number: 1,
        name: "Карбонатный каменный мир — холодный",
        planet_types: &["Rocky body"],
        atmosphere: "Carbon dioxide",
        temperature: (190.0, 195.0),
        pressure: (0.042, 0.237),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::None,
        max_expensive_species: 9,
        potential_max_credits: 363_000_000,
    },
    SearchProfile {
        number: 2,
        name: "Карбонатный каменный мир",
        planet_types: &["Rocky body"],
        atmosphere: "Carbon dioxide",
        temperature: (180.0, 190.0),
        pressure: (0.042, 0.275),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::None,
        max_expensive_species: 7,
        potential_max_credits: 166_000_000,
    },
    SearchProfile {
        number: 3,
        name: "Водный каменный / металлический мир",
        planet_types: &["Rocky body", "High metal content world"],
        atmosphere: "Water",
        temperature: (395.0, 450.0),
        pressure: (0.040, 0.063),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::None,
        max_expensive_species: 6,
        potential_max_credits: 279_000_000,
    },
    SearchProfile {
        number: 4,
        name: "Аммиачный каменный мир",
        planet_types: &["Rocky body"],
        atmosphere: "Ammonia",
        temperature: (165.0, 175.0),
        pressure: (0.040, 0.340),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::Any,
        max_expensive_species: 4,
        potential_max_credits: 94_000_000,
    },
    SearchProfile {
        number: 5,
        name: "Аргоновый ледяной мир",
        planet_types: &["Icy body"],
        atmosphere: "Argon",
        temperature: (50.0, 150.0),
        pressure: (0.167, 0.272),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::WaterMagmaAndGeysers,
        max_expensive_species: 3,
        potential_max_credits: 65_000_000,
    },
    SearchProfile {
        number: 6,
        name: "Сернистый каменный мир",
        planet_types: &["Rocky body"],
        atmosphere: "Sulphur dioxide",
        temperature: (132.0, 195.0),
        pressure: (0.044, 0.275),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::Any,
        max_expensive_species: 2,
        potential_max_credits: 141_000_000,
    },
    SearchProfile {
        number: 7,
        name: "Кислородный ледяной мир",
        planet_types: &["Icy body"],
        atmosphere: "Oxygen",
        temperature: (145.0, 270.0),
        pressure: (0.238, 0.275),
        gravity: (0.1, 1.0),
        volcanism: VolcanismFilter::Any,
        max_expensive_species: 2,
        potential_max_credits: 111_000_000,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_seven_profiles_keep_the_specified_ranges_and_order() {
        let expected = [
            (
                1,
                "Carbon dioxide",
                (190.0, 195.0),
                (0.042, 0.237),
                9,
                363_000_000,
            ),
            (
                1,
                "Carbon dioxide",
                (180.0, 190.0),
                (0.042, 0.275),
                7,
                166_000_000,
            ),
            (2, "Water", (395.0, 450.0), (0.040, 0.063), 6, 279_000_000),
            (1, "Ammonia", (165.0, 175.0), (0.040, 0.340), 4, 94_000_000),
            (1, "Argon", (50.0, 150.0), (0.167, 0.272), 3, 65_000_000),
            (
                1,
                "Sulphur dioxide",
                (132.0, 195.0),
                (0.044, 0.275),
                2,
                141_000_000,
            ),
            (1, "Oxygen", (145.0, 270.0), (0.238, 0.275), 2, 111_000_000),
        ];
        for (index, (profile, (atmosphere, temp, pressure, species, credits))) in
            SEARCH_PROFILES.iter().zip(expected).enumerate()
        {
            assert_eq!(profile.planet_types, expected_types[index]);
            assert_eq!(profile.atmosphere, atmosphere);
            assert_eq!(profile.temperature, temp);
            assert_eq!(profile.pressure, pressure);
            assert_eq!(profile.gravity, (0.1, 1.0));
            assert_eq!(profile.max_expensive_species, species);
            assert_eq!(profile.potential_max_credits, credits);
            assert_eq!(profile.criteria(5).unwrap().count, 5);
        }
    }
}
