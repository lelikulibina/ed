use crate::{
    application::{
        config::{SearchConfig, SearchConfigError},
        presets::{PRESET_GRAVITY_RANGE, SearchPreset},
    },
    cli::{parse_range_f64, parse_range_i32},
    domain::{
        catalog::{AtmosphereType, SurfaceType},
        criteria::{CriteriaError, Filter, NumericRange, SearchCriteria},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    System,
    Surface,
    Atmosphere,
    Temperature,
    Gravity,
    Pressure,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct FieldError {
    pub field: FormField,
    pub message: String,
}

impl FieldError {
    fn new(field: FormField, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchForm {
    pub system: String,
    pub surface_types: Vec<SurfaceType>,
    pub atmosphere_types: Vec<AtmosphereType>,
    pub temperature: String,
    pub gravity: String,
    pub pressure: String,
    pub count: String,
    pub exclude_with_bio_signals: bool,
}

impl Default for SearchForm {
    fn default() -> Self {
        Self {
            system: String::new(),
            surface_types: vec![SurfaceType::Any],
            atmosphere_types: vec![AtmosphereType::Any],
            temperature: "0-5000".to_owned(),
            gravity: "0.1-1.0".to_owned(),
            pressure: "0-10".to_owned(),
            count: "5".to_owned(),
            exclude_with_bio_signals: false,
        }
    }
}

impl SearchForm {
    pub fn from_config(config: &SearchConfig) -> Self {
        let criteria = &config.criteria;
        let surface_types = match &criteria.surface {
            Filter::Any => vec![SurfaceType::Any],
            Filter::OneOf(values) => values.clone(),
        };
        let atmosphere_types = match &criteria.atmosphere {
            Filter::Any => vec![AtmosphereType::Any],
            Filter::OneOf(values) => values.clone(),
        };

        Self {
            system: config.system.clone(),
            surface_types,
            atmosphere_types,
            temperature: format_range(criteria.temperature),
            gravity: format_range(criteria.gravity),
            pressure: format_range(criteria.pressure),
            count: criteria.count.to_string(),
            exclude_with_bio_signals: criteria.exclude_with_bio_signals,
        }
    }

    pub fn apply_preset(&mut self, preset: &SearchPreset) {
        self.surface_types = preset.surface_types.to_vec();
        self.atmosphere_types = vec![preset.atmosphere];
        self.temperature = format_range(preset.temperature);
        self.gravity = format_range(PRESET_GRAVITY_RANGE);
        self.pressure = format_range(preset.pressure);
    }

    pub fn summary(&self) -> String {
        let surfaces = labels_surface(&self.surface_types);
        let atmospheres = labels_atmosphere(&self.atmosphere_types);
        format!(
            "Система: {}\nПоверхность: {}\nАтмосфера: {}\nТемпература: {} K\nГравитация: {} g\nДавление: {} atm\nКоличество: {}\nИсключать биосигналы: {}",
            self.system.trim(),
            surfaces,
            atmospheres,
            self.temperature,
            self.gravity,
            self.pressure,
            self.count,
            if self.exclude_with_bio_signals {
                "да"
            } else {
                "нет"
            }
        )
    }

    pub fn build(&self) -> Result<SearchConfig, Vec<FieldError>> {
        let mut errors = Vec::new();

        let system = self.system.trim().to_owned();
        if system.is_empty() {
            errors.push(FieldError::new(
                FormField::System,
                "Укажите центральную систему",
            ));
        }

        let count = match self.count.trim().parse::<usize>() {
            Ok(0) => {
                errors.push(FieldError::new(
                    FormField::Count,
                    "Количество результатов должно быть больше нуля",
                ));
                1
            }
            Ok(count) => count,
            Err(_) => {
                errors.push(FieldError::new(
                    FormField::Count,
                    "Количество результатов должно быть целым числом",
                ));
                1
            }
        };

        let temperature = match parse_integer_range(
            &self.temperature,
            FormField::Temperature,
            "Температура: ожидается диапазон min-max в K",
            "Некорректный диапазон температуры",
        ) {
            Ok(range) => range,
            Err(error) => {
                errors.push(error);
                NumericRange {
                    min: 0.0,
                    max: 5_000.0,
                }
            }
        };
        let gravity = match parse_float_range(
            &self.gravity,
            FormField::Gravity,
            "Гравитация: ожидается диапазон min-max в g",
            "Некорректная гравитация",
        ) {
            Ok(range) => range,
            Err(error) => {
                errors.push(error);
                NumericRange { min: 0.1, max: 1.0 }
            }
        };
        let pressure = match parse_float_range(
            &self.pressure,
            FormField::Pressure,
            "Давление: ожидается диапазон min-max в atm",
            "Некорректное давление",
        ) {
            Ok(range) => range,
            Err(error) => {
                errors.push(error);
                NumericRange {
                    min: 0.0,
                    max: 10.0,
                }
            }
        };

        if !errors.is_empty() {
            return Err(errors);
        }

        let surface =
            if self.surface_types.is_empty() || self.surface_types.contains(&SurfaceType::Any) {
                Filter::Any
            } else {
                Filter::one_of(self.surface_types.clone())
            };
        let atmosphere = if self.atmosphere_types.is_empty()
            || self.atmosphere_types.contains(&AtmosphereType::Any)
        {
            Filter::Any
        } else {
            Filter::one_of(self.atmosphere_types.clone())
        };

        let criteria =
            SearchCriteria::new(surface, atmosphere, temperature, gravity, pressure, count)
                .map_err(|error| vec![criteria_error(error)])?
                .with_bio_signal_exclusion(self.exclude_with_bio_signals);

        SearchConfig::new(system, criteria).map_err(|error| vec![config_error(error)])
    }
}

fn parse_integer_range(
    value: &str,
    field: FormField,
    parse_message: &str,
    range_message: &str,
) -> Result<NumericRange, FieldError> {
    let (min, max) = parse_range_i32(value).map_err(|_| FieldError::new(field, parse_message))?;
    NumericRange::new(f64::from(min), f64::from(max))
        .ok_or_else(|| FieldError::new(field, range_message))
}

fn parse_float_range(
    value: &str,
    field: FormField,
    parse_message: &str,
    range_message: &str,
) -> Result<NumericRange, FieldError> {
    let (min, max) = parse_range_f64(value).map_err(|_| FieldError::new(field, parse_message))?;
    NumericRange::new(min, max).ok_or_else(|| FieldError::new(field, range_message))
}

fn format_range(range: NumericRange) -> String {
    format!("{}-{}", range.min, range.max)
}

fn labels_surface(values: &[SurfaceType]) -> String {
    if values.is_empty() || values.contains(&SurfaceType::Any) {
        return SurfaceType::Any.label_ru().to_owned();
    }
    values
        .iter()
        .map(|value| value.label_ru())
        .collect::<Vec<_>>()
        .join(", ")
}

fn labels_atmosphere(values: &[AtmosphereType]) -> String {
    if values.is_empty() || values.contains(&AtmosphereType::Any) {
        return AtmosphereType::Any.label_ru().to_owned();
    }
    values
        .iter()
        .map(|value| value.label_ru())
        .collect::<Vec<_>>()
        .join(", ")
}

fn criteria_error(error: CriteriaError) -> FieldError {
    match error {
        CriteriaError::Count => FieldError::new(
            FormField::Count,
            "Количество результатов должно быть больше нуля",
        ),
    }
}

fn config_error(error: SearchConfigError) -> FieldError {
    match error {
        SearchConfigError::EmptySystem => {
            FieldError::new(FormField::System, "Укажите центральную систему")
        }
        SearchConfigError::ZeroCount => FieldError::new(
            FormField::Count,
            "Количество результатов должно быть больше нуля",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_form_builds_unfiltered_criteria() {
        let form = SearchForm {
            system: " Sol ".to_owned(),
            ..SearchForm::default()
        };

        let config = form.build().unwrap();
        assert_eq!(config.system, "Sol");
        assert_eq!(config.criteria.surface, Filter::Any);
        assert_eq!(config.criteria.atmosphere, Filter::Any);
        assert_eq!(config.criteria.count, 5);
    }

    #[test]
    fn from_config_round_trips_typed_filters_and_ranges() {
        let criteria = SearchCriteria::new(
            Filter::one_of(vec![
                SurfaceType::RockyBody,
                SurfaceType::HighMetalContentWorld,
            ]),
            Filter::one_of(vec![AtmosphereType::Water, AtmosphereType::Nitrogen]),
            NumericRange {
                min: 395.0,
                max: 450.0,
            },
            NumericRange { min: 0.1, max: 1.0 },
            NumericRange {
                min: 0.040,
                max: 0.063,
            },
            12,
        )
        .unwrap()
        .with_bio_signal_exclusion(true);
        let config = SearchConfig::new("Sol", criteria).unwrap();

        assert_eq!(SearchForm::from_config(&config).build().unwrap(), config);
    }

    #[test]
    fn applying_preset_preserves_system_count_and_bio_flag() {
        let mut form = SearchForm {
            system: "Sol".to_owned(),
            count: "9".to_owned(),
            exclude_with_bio_signals: true,
            ..SearchForm::default()
        };
        form.apply_preset(&crate::application::presets::PRESETS[2]);

        let config = form.build().unwrap();
        assert_eq!(
            config.criteria.surface,
            Filter::one_of(vec![
                SurfaceType::RockyBody,
                SurfaceType::HighMetalContentWorld,
            ])
        );
        assert_eq!(
            config.criteria.atmosphere,
            Filter::one_of(vec![AtmosphereType::Water])
        );
        assert_eq!(
            config.criteria.temperature,
            NumericRange {
                min: 395.0,
                max: 450.0
            }
        );
        assert_eq!(
            config.criteria.pressure,
            NumericRange {
                min: 0.040,
                max: 0.063
            }
        );
        assert_eq!(config.criteria.gravity, PRESET_GRAVITY_RANGE);
        assert_eq!(config.criteria.count, 9);
        assert!(config.criteria.exclude_with_bio_signals);
    }

    #[test]
    fn build_returns_errors_bound_to_invalid_fields() {
        let form = SearchForm {
            system: "  ".to_owned(),
            temperature: "invalid".to_owned(),
            gravity: "1.0-0.1".to_owned(),
            count: "0".to_owned(),
            ..SearchForm::default()
        };

        let errors = form.build().unwrap_err();
        assert!(errors.iter().any(|error| error.field == FormField::System));
        assert!(
            errors
                .iter()
                .any(|error| error.field == FormField::Temperature)
        );
        assert!(errors.iter().any(|error| error.field == FormField::Gravity));
        assert!(errors.iter().any(|error| error.field == FormField::Count));
    }

    #[test]
    fn summary_uses_russian_catalog_labels() {
        let form = SearchForm {
            surface_types: vec![SurfaceType::RockyBody, SurfaceType::HighMetalContentWorld],
            atmosphere_types: vec![AtmosphereType::Water],
            ..SearchForm::default()
        };

        let summary = form.summary();
        assert!(summary.contains("Каменистая"));
        assert!(summary.contains("С высоким содержанием металлов"));
        assert!(summary.contains("Водная"));
    }
}
