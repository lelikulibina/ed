use serde_json::{Value, json};

use ed::domain::body::{Body, atmosphere_filter_labels};
use ed::spansh::{SpanshClient, SystemResolver};

fn page_count(response: &Value) -> Option<u64> {
    response.get("count").and_then(Value::as_u64)
}

fn results(response: &Value) -> &[Value] {
    response.get("results").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

#[tokio::test]
#[ignore = "requires live network; targeted Spansh water-atmosphere diagnosis"]
async fn live_spansh_water_atmosphere_diagnostic() {
    let client = SpanshClient::spansh(1).unwrap();
    let center = SystemResolver::new(client.clone())
        .resolve("Drojia ZB-Z b32-0")
        .await
        .expect("Drojia center system must resolve");
    eprintln!(
        "WATER_DIAGNOSTIC center={} id64={} coords=({},{},{})",
        center.name, center.id64, center.coordinates.x, center.coordinates.y, center.coordinates.z
    );

    let base = json!({
        "filters": { "distance": { "min": 0, "max": 1000 } },
        "sort": [{ "distance": { "direction": "asc" } }],
        "size": 100,
        "page": 0,
        "reference_coords": {
            "x": center.coordinates.x,
            "y": center.coordinates.y,
            "z": center.coordinates.z
        }
    });

    let mut water_payload = base.clone();
    water_payload["filters"]["atmosphere"] = json!({
        "value": atmosphere_filter_labels("Water")
    });
    let water = client.post_json("/api/bodies/search", &water_payload).await.unwrap();
    let water_rows = results(&water);
    eprintln!(
        "WATER_DIAGNOSTIC query=Water_variants labels={:?} count={:?} page_rows={}",
        water_payload["filters"]["atmosphere"]["value"],
        page_count(&water),
        water_rows.len()
    );
    for row in water_rows.iter().take(15) {
        let parsed = Body::from_json(row).ok();
        eprintln!(
            "WATER_ROW raw_atmosphere={:?} composition={:?} parsed={:?} subtype={:?} landable={:?} mapped={:?} temp={:?} pressure={:?} gravity={:?} system={:?}",
            row.get("atmosphere"),
            row.get("atmosphere_composition"),
            parsed.as_ref().and_then(|body| body.atmosphere.as_deref()),
            parsed.as_ref().and_then(|body| body.subtype.as_deref()),
            parsed.as_ref().and_then(|body| body.is_landable),
            parsed.as_ref().and_then(|body| body.is_mapped),
            parsed.as_ref().and_then(|body| body.surface_temperature),
            parsed.as_ref().and_then(|body| body.surface_pressure),
            parsed.as_ref().and_then(|body| body.gravity),
            parsed.as_ref().and_then(|body| body.system_name.as_deref())
        );
    }

    let mut water_rich_payload = base.clone();
    water_rich_payload["filters"]["atmosphere"] = json!({ "value": ["Water-rich"] });
    let water_rich = client.post_json("/api/bodies/search", &water_rich_payload).await.unwrap();
    eprintln!(
        "WATER_DIAGNOSTIC query=Water-rich count={:?} page_rows={} first_raw={:?}",
        page_count(&water_rich),
        results(&water_rich).len(),
        results(&water_rich).first()
    );

    let mut exact_water_payload = base;
    exact_water_payload["filters"]["atmosphere"] = json!({ "value": ["Water"] });
    let exact_water = client.post_json("/api/bodies/search", &exact_water_payload).await.unwrap();
    eprintln!(
        "WATER_DIAGNOSTIC query=exact_Water count={:?} page_rows={} first_raw={:?}",
        page_count(&exact_water),
        results(&exact_water).len(),
        results(&exact_water).first()
    );

    assert!(
        water.get("results").and_then(Value::as_array).is_some(),
        "Spansh Water variants response must contain results array"
    );
}
