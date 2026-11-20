use std::collections::BTreeMap;

use crate::domain::body::Body;

pub fn print_results(bodies: &[Body]) {
    if bodies.is_empty() {
        println!("❌ Не найдено подходящих планет.");
        return;
    }

    let mut systems: BTreeMap<(String, String), Vec<&Body>> = BTreeMap::new();
    for body in bodies {
        let system_id = body.system_id64.clone().unwrap_or_else(|| {
            body.system_name
                .clone()
                .unwrap_or_else(|| "unknown".to_owned())
        });
        let system_name = body
            .system_name
            .clone()
            .unwrap_or_else(|| format!("Система {system_id}"));
        systems
            .entry((system_name, system_id))
            .or_default()
            .push(body);
    }

    println!("\nНайденные планеты по системам:\n");
    for ((system_name, system_id), planets) in systems {
        println!("Система: {system_name} (ID64: {system_id})");
        for body in planets {
            println!(
                "  • {}{}\n    Тип: {}\n    Атмосфера: {}\n    Температура: {} K\n    Давление: {}\n    Гравитация: {} g\n    Расстояние от центра поиска: {} св. л.\n",
                body.name,
                if body.is_mapped == Some(true) {
                    " [изучена]"
                } else {
                    ""
                },
                body.subtype.as_deref().unwrap_or("неизвестно"),
                body.atmosphere.as_deref().unwrap_or("неизвестно"),
                display_number(body.surface_temperature),
                display_number(body.surface_pressure),
                display_number(body.gravity),
                display_number(body.distance_ly),
            );
        }
    }
}

fn display_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "нет данных".to_owned())
}
