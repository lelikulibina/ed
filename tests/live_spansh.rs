#![allow(dead_code)]

use ed_planet_search::{
    domain::criteria::{Filter, NumericRange, SearchCriteria},
    spansh::{BodySearch, SpanshClient, SystemResolver},
};

/// Ручной smoke-тест публичного контракта поиска Spansh.
/// Запуск: cargo test --test live_spansh -- --ignored --nocapture
#[tokio::test]
#[ignore = "requires live network; manual Spansh contract smoke test"]
async fn live_spansh_body_search_contract() {
    let client = SpanshClient::spansh(1).expect("Spansh client must initialize");
    let center = SystemResolver::new(client.clone())
        .resolve("Sol")
        .await
        .expect("Spansh must resolve Sol");
    assert_eq!(center.name, "Sol");

    let criteria = SearchCriteria::new(
        Filter::Any,
        Filter::Any,
        NumericRange {
            min: 0.0,
            max: 5_000.0,
        },
        NumericRange {
            min: 0.0,
            max: 10.0,
        },
        NumericRange {
            min: 0.0,
            max: 1_000.0,
        },
        1,
    )
    .expect("broad smoke-test criteria must be valid");

    let results = BodySearch::new(client)
        .search(&center, &criteria)
        .await
        .expect("Spansh body search must complete");

    assert!(
        !results.is_empty(),
        "Sol neighborhood must yield a matching body"
    );
    assert!(
        results.len() <= 1,
        "search must honor the requested result count"
    );
    assert!(
        results.iter().all(|body| criteria.matches(body)),
        "every returned body must satisfy the local criteria"
    );
    assert!(
        results.iter().all(|body| body
            .distance_ly
            .is_some_and(|distance| { (0.0..=1_000.0).contains(&distance) })),
        "every returned body must be within the fixed 1000-ly search radius"
    );
}

/// Диагностика разрешения Flyua Phoea WB-K b1 и геометрии расстояний.
/// Сверяет найденный центр с точным именем, координатами /api/system/{id64},
/// а расстояния выдачи — с расстояниями, рассчитанными по координатам систем.
#[tokio::test]
#[ignore = "requires live Spansh API; temporary Flyua Phoea distance investigation"]
async fn live_flyua_phoea_center_and_distances_are_consistent() {
    let requested = "Flyua Phoea WB-K b1";
    let client = SpanshClient::spansh(1).expect("Spansh client must initialize");
    let center = SystemResolver::new(client.clone())
        .resolve(requested)
        .await
        .expect("Spansh must resolve the requested Flyua Phoea system");

    println!(
        "requested={requested:?}; resolved={:?}; id64={}; coords=({}, {}, {})",
        center.name,
        center.id64,
        center.coordinates.x,
        center.coordinates.y,
        center.coordinates.z
    );
    assert!(
        center.name.trim().eq_ignore_ascii_case(requested),
        "resolver selected a different system: requested {requested:?}, got {:?}",
        center.name
    );

    let center_detail = client
        .get_json(&format!("/api/system/{}", center.id64))
        .await
        .expect("Spansh must return details for the resolved center");
    let center_root = center_detail.get("system").unwrap_or(&center_detail);
    let center_coords = center_root
        .get("coords")
        .or_else(|| center_root.get("coordinates"))
        .unwrap_or(center_root);
    let detail_coordinates = (
        center_coords["x"].as_f64().expect("center x coordinate"),
        center_coords["y"].as_f64().expect("center y coordinate"),
        center_coords["z"].as_f64().expect("center z coordinate"),
    );
    let center_delta = (
        center.coordinates.x - detail_coordinates.0,
        center.coordinates.y - detail_coordinates.1,
        center.coordinates.z - detail_coordinates.2,
    );
    println!(
        "detail_coords={detail_coordinates:?}; resolver_delta={center_delta:?}"
    );
    assert!(
        center_delta.0.abs() < 0.001
            && center_delta.1.abs() < 0.001
            && center_delta.2.abs() < 0.001,
        "resolver coordinates differ from /api/system/{}/ coordinates",
        center.id64
    );

    let criteria = SearchCriteria::new(
        Filter::Any,
        Filter::Any,
        NumericRange {
            min: 0.0,
            max: 5_000.0,
        },
        NumericRange {
            min: 0.0,
            max: 1_000.0,
        },
        NumericRange {
            min: 0.0,
            max: 1_000.0,
        },
        5,
    )
    .expect("broad diagnostic criteria must be valid");
    let results = BodySearch::new(client.clone())
        .search(&center, &criteria)
        .await
        .expect("Spansh body search around Flyua Phoea must complete");
    assert!(!results.is_empty(), "diagnostic search must return bodies");

    let mut checked = 0;
    for body in results {
        let Some(system_id64) = body.system_id64.as_deref() else {
            println!("skip body {:?}: missing parent system ID64", body.name);
            continue;
        };
        let reported = body
            .distance_ly
            .expect("Spansh search result must include distance");
        let detail = client
            .get_json(&format!("/api/system/{system_id64}"))
            .await
            .expect("Spansh must return details for each result system");
        let root = detail.get("system").unwrap_or(&detail);
        let coords = root
            .get("coords")
            .or_else(|| root.get("coordinates"))
            .unwrap_or(root);
        let x = coords["x"].as_f64().expect("result system x coordinate");
        let y = coords["y"].as_f64().expect("result system y coordinate");
        let z = coords["z"].as_f64().expect("result system z coordinate");
        let dx = x - center.coordinates.x;
        let dy = y - center.coordinates.y;
        let dz = z - center.coordinates.z;
        let calculated = (dx * dx + dy * dy + dz * dz).sqrt();
        println!(
            "body={:?}; system_id64={system_id64}; reported_distance={reported}; calculated_distance={calculated}",
            body.name
        );
        assert!(
            (reported - calculated).abs() <= 0.5,
            "distance mismatch for system {system_id64}: Spansh reports {reported} ly, coordinates give {calculated} ly"
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "at least one returned body must have a parent system ID64 to verify its distance"
    );
}
