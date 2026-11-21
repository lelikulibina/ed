use std::cmp::Ordering;

use crate::domain::body::Body;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Surface,
    Atmosphere,
    Temperature,
    Pressure,
    Gravity,
    Distance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

pub fn format_optional(value: Option<f64>, precision: usize, suffix: &str) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => format!("{value:.precision$}{suffix}"),
        None => "—".to_owned(),
    }
}

fn compare_optional<T: Ord>(
    left: Option<T>,
    right: Option<T>,
    direction: SortDirection,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            let order = left.cmp(&right);
            match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_number(
    left: Option<f64>,
    right: Option<f64>,
    direction: SortDirection,
) -> Ordering {
    compare_optional(
        left.filter(|value| value.is_finite()).map(f64::to_bits),
        right.filter(|value| value.is_finite()).map(f64::to_bits),
        direction,
    )
    .then_with(|| {
        match (
            left.filter(|value| value.is_finite()),
            right.filter(|value| value.is_finite()),
        ) {
            (Some(left), Some(right)) => {
                let order = left.total_cmp(&right);
                match direction {
                    SortDirection::Ascending => order,
                    SortDirection::Descending => order.reverse(),
                }
            }
            _ => Ordering::Equal,
        }
    })
}

fn compare_text(
    left: Option<&str>,
    right: Option<&str>,
    direction: SortDirection,
) -> Ordering {
    compare_optional(
        left.map(str::to_lowercase),
        right.map(str::to_lowercase),
        direction,
    )
}

pub fn sort_bodies(bodies: &mut [Body], column: SortColumn, direction: SortDirection) {
    bodies.sort_by(|left, right| match column {
        SortColumn::Name => compare_text(Some(&left.name), Some(&right.name), direction),
        SortColumn::Surface => {
            compare_text(left.subtype.as_deref(), right.subtype.as_deref(), direction)
        }
        SortColumn::Atmosphere => {
            compare_text(left.atmosphere.as_deref(), right.atmosphere.as_deref(), direction)
        }
        SortColumn::Temperature => compare_optional_number(
            left.surface_temperature,
            right.surface_temperature,
            direction,
        ),
        SortColumn::Pressure => {
            compare_optional_number(left.surface_pressure, right.surface_pressure, direction)
        }
        SortColumn::Gravity => {
            compare_optional_number(left.gravity, right.gravity, direction)
        }
        SortColumn::Distance => {
            compare_optional_number(left.distance_ly, right.distance_ly, direction)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{SortColumn, SortDirection, format_optional, sort_bodies};

    use crate::domain::body::Body;

    fn body(
        name: &str,
        surface: Option<&str>,
        atmosphere: Option<&str>,
        temperature: Option<f64>,
        pressure: Option<f64>,
        gravity: Option<f64>,
        distance: Option<f64>,
    ) -> Body {
        Body {
            id64: None,
            name: name.to_owned(),
            system_id64: None,
            system_name: None,
            subtype: surface.map(str::to_owned),
            distance_ly: distance,
            is_landable: None,
            is_mapped: None,
            last_updated_at: None,
            surface_temperature: temperature,
            surface_pressure: pressure,
            gravity,
            atmosphere: atmosphere.map(str::to_owned),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn optional_numbers_preserve_precision_and_units() {
        assert_eq!(format_optional(Some(190.0), 2, " K"), "190.00 K");
        assert_eq!(format_optional(Some(0.042), 3, " atm"), "0.042 atm");
        assert_eq!(format_optional(Some(0.1), 3, " g"), "0.100 g");
        assert_eq!(
            format_optional(Some(12.5), 2, " св. л."),
            "12.50 св. л."
        );
    }

    #[test]
    fn absent_and_non_finite_numbers_are_not_rendered_as_zero() {
        assert_eq!(format_optional(None, 2, " K"), "—");
        assert_eq!(format_optional(Some(f64::NAN), 2, " K"), "—");
        assert_eq!(format_optional(Some(f64::INFINITY), 2, " K"), "—");
    }

    #[test]
    fn sorts_all_columns_in_both_directions_and_keeps_missing_values_last() {
        let mut bodies = vec![
            body(
                "Beta",
                Some("Icy body"),
                Some("Water"),
                Some(200.0),
                Some(0.2),
                Some(0.4),
                Some(20.0),
            ),
            body("Gamma", None, None, None, None, None, None),
            body(
                "Alpha",
                Some("Rocky body"),
                Some("Ammonia"),
                Some(100.0),
                Some(0.1),
                Some(0.2),
                Some(10.0),
            ),
        ];

        for column in [
            SortColumn::Name,
            SortColumn::Surface,
            SortColumn::Atmosphere,
            SortColumn::Temperature,
            SortColumn::Pressure,
            SortColumn::Gravity,
            SortColumn::Distance,
        ] {
            sort_bodies(&mut bodies, column, SortDirection::Ascending);
            assert_eq!(
                bodies.last().unwrap().name,
                "Gamma",
                "ascending {column:?}"
            );

            sort_bodies(&mut bodies, column, SortDirection::Descending);
            assert_eq!(
                bodies.last().unwrap().name,
                "Gamma",
                "descending {column:?}"
            );
        }

        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Ascending);
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta", "Gamma"]
        );
        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Descending);
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Beta", "Alpha", "Gamma"]
        );
    }
}
