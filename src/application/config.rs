use crate::domain::criteria::{NumericRange, SearchCriteria};

#[cfg(test)]
mod tests {
    use super::*;

    fn criteria(count: usize) -> SearchCriteria {
        SearchCriteria::from_cli(
            "Rocky body",
            "Carbon dioxide",
            NumericRange { min: 190.0, max: 195.0 },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange { min: 0.042, max: 0.237 },
            count,
        )
        .unwrap()
    }

    #[test]
    fn accepts_a_trimmed_non_empty_system_and_preserves_criteria() {
        let config = SearchConfig::new("  Sol  ", criteria(7)).unwrap();
        assert_eq!(config.system, "Sol");
        assert_eq!(config.criteria.count, 7);
    }

    #[test]
    fn rejects_an_empty_system_name() {
        assert!(SearchConfig::new("  ", criteria(5)).is_err());
    }

    #[test]
    fn rejects_zero_result_count_even_if_criteria_was_mutated() {
        assert!(SearchConfig::new("Sol", criteria(5).with_count(0).unwrap_err().into()).is_err());
    }
}
