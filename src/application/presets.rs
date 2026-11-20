use crate::domain::criteria::{NumericRange, SearchCriteria};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_the_seven_user_defined_profiles_exactly() {
        let expected = [
            (1, "Rocky body", "Carbon dioxide", 190.0, 195.0, 0.042, 0.237, 9, 363_000_000_u64),
            (2, "Rocky body", "Carbon dioxide", 180.0, 190.0, 0.042, 0.275, 7, 166_000_000_u64),
            (3, "Rocky body|High metal content world", "Water", 395.0, 450.0, 0.040, 0.063, 6, 279_000_000_u64),
            (4, "Rocky body", "Ammonia", 165.0, 175.0, 0.040, 0.340, 4, 94_000_000_u64),
            (5, "Icy body", "Argon", 50.0, 150.0, 0.167, 0.272, 3, 65_000_000_u64),
            (6, "Rocky body", "Sulphur dioxide", 132.0, 195.0, 0.044, 0.275, 2, 141_000_000_u64),
            (7, "Icy body", "Oxygen", 145.0, 270.0, 0.238, 0.275, 2, 111_000_000_u64),
        ];

        assert_eq!(PRESETS.len(), expected.len());
        for (preset, row) in PRESETS.iter().zip(expected) {
            assert_eq!(preset.number, row.0);
            assert_eq!(preset.surface_types.join("|"), row.1);
            assert_eq!(preset.atmosphere, row.2);
            assert_eq!(preset.temperature, NumericRange { min: row.3, max: row.4 });
            assert_eq!(preset.pressure, NumericRange { min: row.5, max: row.6 });
            assert_eq!(preset.expensive_species, row.7);
            assert_eq!(preset.potential_value_cr, row.8);
        }
    }

    #[test]
    fn profile_three_surface_types_are_alternatives_and_volcanism_is_not_filtered() {
        let criteria = PRESETS[2].criteria(5, false).unwrap();
        assert_eq!(
            criteria.planet_types,
            vec!["Rocky body".to_owned(), "High metal content world".to_owned()]
        );
        assert_eq!(criteria.gravity, NumericRange { min: 0.1, max: 1.0 });
        assert_eq!(criteria.count, 5);
    }

    #[test]
    fn profiles_preserve_bio_signal_exclusion_as_a_separate_option() {
        let criteria = PRESETS[0].criteria(5, true).unwrap();
        assert!(criteria.exclude_with_bio_signals);
    }
}
