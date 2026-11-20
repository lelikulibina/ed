use std::collections::BTreeMap;

use crate::domain::body::Body;

pub fn print_results(bodies: &[Body]) {
    if bodies.is_empty() {
        println!("❌ Не найдено подходящих планет.");
        return;
    }

    for (system_id, planets) in group_by_system(bodies) {
        let system_name = planets
            .iter()
            .find_map(|body| body.system_name.as_deref())
            .unwrap_or("неизвестная система");
        println!("Система: {system_name} (ID64: {system_id})");
        for body in planets {
            println!(
                "  • {}{}\n    Тип: {}\n    Атмосфера: {}\n    Температура: {} °C\n    Давление: {}\n    Гравитация: {} g\n    Расстояние от центра поиска: {} св. л.\n",
                body.name,
                if body.is_mapped == Some(true) {
                    " [изучена]"
                } else {
                    ""
                },
                body.subtype.as_deref().unwrap_or("неизвестно"),
                body.atmosphere.as_deref().unwrap_or("неизвестно"),
                display_temperature_celsius(body.surface_temperature),
                display_number(body.surface_pressure),
                display_number(body.gravity),
                display_number(body.distance_ly),
            );
        }
    }
}

fn group_by_system(bodies: &[Body]) -> BTreeMap<String, Vec<&Body>> {
    let mut systems: BTreeMap<String, Vec<&Body>> = BTreeMap::new();
    for body in bodies {
        let system_id = body
            .system_id64
            .clone()
            .or_else(|| body.system_name.clone())
            .unwrap_or_else(|| "unknown".to_owned());
        systems.entry(system_id).or_default().push(body);
    }
    systems
}

fn display_temperature_celsius(value_kelvin: Option<f64>) -> String {
    value_kelvin
        .map(|kelvin| format!("{:.3}", kelvin - 273.15))
        .unwrap_or_else(|| "нет данных".to_owned())
}

fn display_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "нет данных".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(name: &str, system_id64: &str, system_name: &str) -> Body {
        Body {
            id64: None,
            name: name.to_owned(),
            system_id64: Some(system_id64.to_owned()),
            system_name: Some(system_name.to_owned()),
            subtype: None,
            distance_ly: None,
            is_landable: None,
            is_mapped: None,
            surface_temperature: None,
            surface_pressure: None,
            gravity: None,
            atmosphere: None,
        }
    }

    #[test]
    fn groups_by_system_id64_not_system_name() {
        let bodies = [
            body("A 1", "100", "System A"),
            body("A 2", "100", "Different spelling"),
            body("B 1", "200", "System A"),
        ];
        let groups = group_by_system(&bodies);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups["100"].len(), 2);
        assert_eq!(groups["200"].len(), 1);
    }
}
