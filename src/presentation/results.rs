use crate::domain::body::Body;

#[cfg(test)]
mod tests {
    use super::super::{SortColumn, SortDirection, format_optional, sort_bodies};

    use super::*;

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
        assert_eq!(format_optional(Some(12.5), 2, " св. л."), "12.50 св. л.");
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
            body("Beta", Some("Icy body"), Some("Water"), Some(200.0), Some(0.2), Some(0.4), Some(20.0)),
            body("Gamma", None, None, None, None, None, None),
            body("Alpha", Some("Rocky body"), Some("Ammonia"), Some(100.0), Some(0.1), Some(0.2), Some(10.0)),
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
            assert_eq!(bodies.last().unwrap().name, "Gamma", "ascending {column:?}");

            sort_bodies(&mut bodies, column, SortDirection::Descending);
            assert_eq!(bodies.last().unwrap().name, "Gamma", "descending {column:?}");
        }

        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Ascending);
        assert_eq!(bodies.iter().map(|body| body.name.as_str()).collect::<Vec<_>>(), vec!["Alpha", "Beta", "Gamma"]);
        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Descending);
        assert_eq!(bodies.iter().map(|body| body.name.as_str()).collect::<Vec<_>>(), vec!["Beta", "Alpha", "Gamma"]);
    }
}
