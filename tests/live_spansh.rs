#![allow(dead_code)]

#[path = "../src/application/presets.rs"]
pub mod application_presets;

mod application {
    pub use crate::application_presets as presets;
}

#[path = "../src/domain/mod.rs"]
mod domain;
#[allow(unused_imports)]
#[path = "../src/spansh/mod.rs"]
mod spansh;

use domain::criteria::{NumericRange, SearchCriteria};
use spansh::{BodySearch, SpanshClient, SystemResolver};

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

    let criteria = SearchCriteria::from_cli(
        "any",
        "any",
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
