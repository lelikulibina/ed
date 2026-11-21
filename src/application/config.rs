use crate::domain::criteria::SearchCriteria;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchConfig {
    pub system: String,
    pub criteria: SearchCriteria,
}

impl SearchConfig {
    pub fn new(
        system: impl Into<String>,
        criteria: SearchCriteria,
    ) -> Result<Self, SearchConfigError> {
        let system = system.into();
        let system = system.trim();
        if system.is_empty() {
            return Err(SearchConfigError::EmptySystem);
        }
        if criteria.count == 0 {
            return Err(SearchConfigError::ZeroCount);
        }

        Ok(Self {
            system: system.to_owned(),
            criteria,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SearchConfigError {
    #[error("укажите центральную систему")]
    EmptySystem,
    #[error("количество результатов должно быть больше нуля")]
    ZeroCount,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::criteria::NumericRange;

    fn criteria(count: usize) -> SearchCriteria {
        SearchCriteria::from_cli(
            "Rocky body",
            "Carbon dioxide",
            NumericRange {
                min: 190.0,
                max: 195.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.042,
                max: 0.237,
            },
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
        assert_eq!(
            SearchConfig::new("  ", criteria(5)).unwrap_err(),
            SearchConfigError::EmptySystem
        );
    }

    #[test]
    fn rejects_zero_result_count_even_if_criteria_was_mutated() {
        let mut invalid = criteria(5);
        invalid.count = 0;
        assert_eq!(
            SearchConfig::new("Sol", invalid).unwrap_err(),
            SearchConfigError::ZeroCount
        );
    }
}
