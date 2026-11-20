use serde_json::Value;
use std::collections::HashSet;

fn catalog() -> Value {
    let source = std::fs::read_to_string("data/exobiology/species-v1.json")
        .expect("versioned exobiology species catalog must be present");
    serde_json::from_str(&source).expect("species catalog must be valid JSON")
}

#[test]
fn catalog_contains_verified_high_value_species_with_base_payouts() {
    let catalog = catalog();
    assert_eq!(catalog["version"].as_u64(), Some(1));

    let species = catalog["species"].as_array().expect("species array");
    for (genus, name, value) in [
        ("Osseus", "Discus", 12_934_900),
        ("Tussock", "Virgam", 14_313_700),
        ("Recepta", "Deltahedronix", 16_202_800),
        ("Clypeus", "Margaritus", 11_873_200),
        ("Cactoida", "Vermis", 16_202_800),
        ("Bacterium", "Cerbrus", 1_689_800),
    ] {
        assert!(
            species.iter().any(|entry| {
                entry["genus"] == genus
                    && entry["species"] == name
                    && entry["base_value_cr"].as_u64() == Some(value)
            }),
            "missing or incorrect payout for {genus} {name}"
        );
    }
}

#[test]
fn catalog_has_no_duplicate_species_and_records_genus_constraints() {
    let catalog = catalog();
    let species = catalog["species"].as_array().expect("species array");
    let mut identities = HashSet::new();

    for entry in species {
        let genus = entry["genus"].as_str().expect("genus");
        let name = entry["species"].as_str().expect("species");
        assert!(identities.insert(format!("{genus}::{name}")));
        assert!(entry["base_value_cr"].as_u64().is_some());
        assert!(entry["atmospheres"].is_array());
        assert!(entry["planet_types"].is_array());
    }

    let osseus_discus = species
        .iter()
        .find(|entry| entry["genus"] == "Osseus" && entry["species"] == "Discus")
        .expect("Osseus Discus");
    assert!(
        osseus_discus["atmospheres"]
            .as_array()
            .unwrap()
            .contains(&Value::String("Water-rich".into()))
    );
}

#[test]
fn genus_preferences_are_not_encoded_as_hard_planet_type_filters() {
    let catalog = catalog();
    let species = catalog["species"].as_array().expect("species array");

    for (genus, name) in [("Tussock", "Virgam"), ("Cactoida", "Vermis")] {
        let entry = species
            .iter()
            .find(|entry| entry["genus"] == genus && entry["species"] == name)
            .expect("species rule");
        assert!(
            entry["planet_types"].as_array().unwrap().is_empty(),
            "{genus} {name} has no exclusive planet-type requirement in the source table"
        );
    }
}

#[test]
fn volcanism_requirement_uses_the_official_spansh_enum_value() {
    let catalog = catalog();
    let species = catalog["species"].as_array().expect("species array");
    let cerbrus = species
        .iter()
        .find(|entry| entry["genus"] == "Bacterium" && entry["species"] == "Cerbrus")
        .expect("Bacterium Cerbrus");
    assert_eq!(cerbrus["volcanism"], serde_json::json!(["No volcanism"]));
}
