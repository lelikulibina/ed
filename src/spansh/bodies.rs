use std::collections::HashSet;

use serde_json::Value;

use crate::domain::{body::Body, criteria::SearchCriteria};

use super::{
    client::{SpanshClient, SpanshError},
    query::{BodySearchQuery, QueryError},
    system::ResolvedSystem,
};

const DEFAULT_RADIUS_LY: f64 = 1_000.0;
const PAGE_SIZE: u16 = 100;
const MAX_PAGES: u32 = 1_000;
const PROVIDER_RESULT_LIMIT: usize = 10_000;

#[derive(Debug, thiserror::Error)]
pub enum BodySearchError {
    #[error(transparent)]
    Spansh(#[from] SpanshError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error("Spansh вернул ответ неизвестной структуры")]
    InvalidResponse,
    #[error("Spansh вернул некорректный объект планеты: {0}")]
    InvalidBody(String),
    #[error("достигнут предел страниц поиска ({0}); результаты могут быть неполными")]
    PageLimit(u32),
}

pub struct BodySearch {
    client: SpanshClient,
    radius_ly: f64,
    page_size: u16,
    max_pages: u32,
}

impl BodySearch {
    pub fn new(client: SpanshClient) -> Self {
        Self {
            client,
            radius_ly: DEFAULT_RADIUS_LY,
            page_size: PAGE_SIZE,
            max_pages: MAX_PAGES,
        }
    }

    #[cfg(test)]
    pub fn with_limits(mut self, radius_ly: f64, page_size: u16, max_pages: u32) -> Self {
        self.radius_ly = radius_ly;
        self.page_size = page_size;
        self.max_pages = max_pages;
        self
    }

    /// Поиск тел вокруг центральной системы. Критерии передаются в Spansh
    /// для серверного отбора и повторно проверяются локально перед выдачей.
    pub async fn search(
        &self,
        center: &ResolvedSystem,
        criteria: &SearchCriteria,
    ) -> Result<Vec<Body>, BodySearchError> {
        let mut query = BodySearchQuery::new(center.coordinates, self.radius_ly, self.page_size)?;
        let initial = self
            .client
            .post_json(
                "/api/bodies/search",
                &query.to_payload_with_criteria(criteria),
            )
            .await?;

        let mut page = parse_page(&initial)?;
        let mut guid = find_guid(&initial);
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        let mut completed = false;

        for page_index in 0..self.max_pages {
            for body in page.bodies.drain(..) {
                let identity = body.id64.clone().unwrap_or_else(|| {
                    let parent = body
                        .system_id64
                        .as_deref()
                        .or(body.system_name.as_deref())
                        .unwrap_or("");
                    format!("{parent}:{}", body.name)
                });
                if !seen.insert(identity) {
                    continue;
                }

                if !is_center_system(&body, center)
                    && body
                        .distance_ly
                        .is_some_and(|distance| (0.0..=self.radius_ly).contains(&distance))
                    && criteria.matches(&body)
                {
                    results.push(body);
                    if results.len() >= criteria.count {
                        return Ok(results);
                    }
                }
            }

            if is_last_page(&page, page_index, self.page_size) {
                completed = true;
                break;
            }

            if page_index + 1 >= self.max_pages {
                return Err(BodySearchError::PageLimit(self.max_pages));
            }

            let next_page = page_index + 1;
            let response = if let Some(search_guid) = guid.as_deref() {
                let path = format!("/api/bodies/search/recall/{search_guid}/{next_page}");
                match self.client.get_json(&path).await {
                    Ok(response) => response,
                    Err(SpanshError::Status { status, .. })
                        if status == reqwest::StatusCode::NOT_FOUND
                            || status == reqwest::StatusCode::GONE
                            || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
                            || status.is_server_error() =>
                    {
                        // A saved search may expire or recall may be temporarily
                        // unavailable. Direct POST pagination is live-confirmed.
                        query.page = next_page;
                        self.client
                            .post_json(
                                "/api/bodies/search",
                                &query.to_payload_with_criteria(criteria),
                            )
                            .await?
                    }
                    Err(error) => return Err(error.into()),
                }
            } else {
                // Live API probe confirms page=N advances the result set; use it
                // when no search_reference is supplied.
                query.page = next_page;
                self.client
                    .post_json(
                        "/api/bodies/search",
                        &query.to_payload_with_criteria(criteria),
                    )
                    .await?
            };
            page = parse_page(&response)?;
            if let Some(next_guid) = find_guid(&response) {
                guid = Some(next_guid);
            }
        }

        if !completed && self.max_pages > 0 {
            return Err(BodySearchError::PageLimit(self.max_pages));
        }
        Ok(results)
    }
}

struct ParsedPage {
    bodies: Vec<Body>,
    exhausted: bool,
    last_page_size: usize,
    total_count: Option<usize>,
    from: Option<usize>,
}

fn is_center_system(body: &Body, center: &ResolvedSystem) -> bool {
    match body.system_id64.as_deref().filter(|id64| !id64.is_empty()) {
        Some(id64) => id64 == center.id64,
        None => body
            .system_name
            .as_deref()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(center.name.trim())),
    }
}

fn is_last_page(page: &ParsedPage, page_index: u32, page_size: u16) -> bool {
    if page.last_page_size == 0 {
        return true;
    }

    // Spansh may cap the reported count at 10,000. Do not treat that
    // metadata (or a full capped final page) as proof that no later pages
    // exist: probe the next page and stop only when it is actually empty.
    if page
        .total_count
        .is_some_and(|count| count >= PROVIDER_RESULT_LIMIT)
    {
        return false;
    }

    if page.exhausted {
        return true;
    }

    if let Some(total_count) = page.total_count {
        let default_offset = page_index as usize * usize::from(page_size);
        let offset = page.from.unwrap_or(default_offset);
        return offset.saturating_add(page.last_page_size) >= total_count;
    }

    page.last_page_size < usize::from(page_size)
}

fn parse_page(value: &Value) -> Result<ParsedPage, BodySearchError> {
    let array = ["results", "bodies", "data", "items"]
        .iter()
        .find_map(|key| {
            value.get(key).and_then(|candidate| {
                candidate.as_array().or_else(|| {
                    ["results", "bodies", "items"]
                        .iter()
                        .find_map(|nested| candidate.get(nested).and_then(Value::as_array))
                })
            })
        })
        .or_else(|| value.as_array())
        .ok_or(BodySearchError::InvalidResponse)?;

    let mut bodies = Vec::with_capacity(array.len());
    for item in array {
        let body = Body::from_json(item)
            .map_err(|error| BodySearchError::InvalidBody(error.to_string()))?;
        bodies.push(body);
    }

    let exhausted = array.is_empty()
        || value.get("has_more").and_then(Value::as_bool) == Some(false)
        || value.get("has_next").and_then(Value::as_bool) == Some(false);

    Ok(ParsedPage {
        bodies,
        exhausted,
        last_page_size: array.len(),
        total_count: value
            .get("count")
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok()),
        from: value
            .get("from")
            .and_then(Value::as_u64)
            .and_then(|from| usize::try_from(from).ok()),
    })
}

fn find_guid(value: &Value) -> Option<String> {
    ["guid", "search_guid", "search_id", "search_reference", "id"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str).map(str::to_owned))
        .or_else(|| {
            value.get("search").and_then(|search| {
                ["guid", "id"]
                    .iter()
                    .find_map(|key| search.get(key).and_then(Value::as_str).map(str::to_owned))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spansh::test_support::{MockResponse, MockServer};

    #[test]
    fn default_search_radius_is_1000_light_years() {
        assert_eq!(DEFAULT_RADIUS_LY, 1_000.0);
    }

    #[test]
    fn full_page_at_reported_10000_count_is_not_treated_as_final() {
        let page = ParsedPage {
            bodies: Vec::new(),
            exhausted: false,
            last_page_size: 100,
            total_count: Some(10_000),
            from: Some(9_900),
        };

        assert!(!is_last_page(&page, 99, 100));
    }

    #[test]
    fn identifies_center_system_by_id64_and_falls_back_to_name() {
        let center = ResolvedSystem {
            id64: "42".to_owned(),
            name: "Sol".to_owned(),
            coordinates: super::super::query::Coordinates {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let mut body = Body {
            id64: Some("100".to_owned()),
            name: "Sol 1".to_owned(),
            system_id64: Some("42".to_owned()),
            system_name: Some("Different label".to_owned()),
            subtype: None,
            distance_ly: Some(0.0),
            is_landable: Some(true),
            is_mapped: None,
            last_updated_at: None,
            surface_temperature: None,
            surface_pressure: None,
            gravity: None,
            atmosphere: None,
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        };

        assert!(is_center_system(&body, &center));
        body.system_id64 = None;
        body.system_name = Some("sOL".to_owned());
        assert!(is_center_system(&body, &center));
        body.system_id64 = Some("43".to_owned());
        assert!(!is_center_system(&body, &center));
    }

    #[test]
    fn parses_body_page_and_marks_empty_page_exhausted() {
        let response = serde_json::json!({
            "guid": "abc-123",
            "results": [{
                "name": "Sol 3",
                "id64": "9223372036854775807",
                "system_id64": "42",
                "subtype": "Earth-like world",
                "is_landable": true,
                "is_mapped": false
            }]
        });
        let page = parse_page(&response).unwrap();
        assert_eq!(page.bodies.len(), 1);
        assert_eq!(page.bodies[0].id64.as_deref(), Some("9223372036854775807"));
        assert!(!page.exhausted);
        assert_eq!(find_guid(&response).as_deref(), Some("abc-123"));

        let empty = parse_page(&serde_json::json!({"results":[]})).unwrap();
        assert!(empty.exhausted);
    }

    #[tokio::test]
    async fn search_posts_center_and_follows_recall_pages() {
        let first = include_str!("../../tests/fixtures/spansh/body-search-page-0.json").to_owned();
        let second = include_str!("../../tests/fixtures/spansh/body-search-page-1.json").to_owned();
        let server =
            MockServer::start(vec![MockResponse::ok(first), MockResponse::ok(second)]).await;

        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let center = ResolvedSystem {
            id64: "42".to_owned(),
            name: "Sol".to_owned(),
            coordinates: super::super::query::Coordinates {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };
        let criteria = SearchCriteria::from_cli(
            "rocky body",
            "nitrogen",
            crate::domain::criteria::NumericRange {
                min: 200.0,
                max: 300.0,
            },
            crate::domain::criteria::NumericRange { min: 0.5, max: 1.5 },
            crate::domain::criteria::NumericRange { min: 0.0, max: 2.0 },
            1,
        )
        .unwrap();

        let results = BodySearch::new(client)
            .with_limits(100.0, 1, 3)
            .search(&center, &criteria)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Nearby B 1");
        let requests = server.finish().await;
        assert!(requests[0].starts_with("POST /api/bodies/search "));
        let (_, request_body) = requests[0].split_once("\r\n\r\n").unwrap();
        let payload: Value = serde_json::from_str(request_body).unwrap();
        assert_eq!(payload["reference_coords"]["x"], 1.0);
        assert_eq!(payload["reference_coords"]["y"], 2.0);
        assert_eq!(payload["reference_coords"]["z"], 3.0);
        assert_eq!(
            payload["filters"]["subtype"]["value"],
            serde_json::json!(["Rocky body"])
        );
        let atmosphere_values = payload["filters"]["atmosphere"]["value"]
            .as_array()
            .unwrap();
        assert!(atmosphere_values.contains(&serde_json::json!("Nitrogen")));
        assert!(atmosphere_values.contains(&serde_json::json!("Thin Nitrogen")));
        assert_eq!(
            payload["filters"]["surface_temperature"]["value"],
            serde_json::json!([200.0, 300.0])
        );
        assert_eq!(payload["filters"]["is_landable"]["value"], true);
        assert!(requests[1].starts_with("GET /api/bodies/search/recall/test-guid/1 "));
    }
    #[tokio::test]
    async fn search_uses_post_page_fallback_when_reference_is_missing() {
        let responses = [
            r#"{"count":2,"from":0,"size":1,"results":[{
                "name":"Central Body",
                "id64":"1001",
                "system_id64":"42",
                "system_name":"Sol",
                "subtype":"Rocky body",
                "is_landable":true,
                "is_mapped":false,
                "surface_temperature":250,
                "surface_pressure":1.0,
                "gravity":1.0,
                "atmosphere":"Nitrogen",
                "distance":1.0
            }]}"#,
            r#"{"count":2,"from":1,"size":1,"results":[{
                "name":"Unmapped Body",
                "id64":"1002",
                "system_id64":"2002",
                "system_name":"Nearby B",
                "subtype":"Rocky body",
                "is_landable":true,
                "is_mapped":false,
                "surface_temperature":250,
                "surface_pressure":1.0,
                "gravity":1.0,
                "atmosphere":"Nitrogen",
                "distance":2.0
            }]}"#,
        ];
        let server = MockServer::start(responses.into_iter().map(MockResponse::ok).collect()).await;

        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let center = ResolvedSystem {
            id64: "42".to_owned(),
            name: "Sol".to_owned(),
            coordinates: super::super::query::Coordinates {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };
        let criteria = SearchCriteria::from_cli(
            "rocky body",
            "nitrogen",
            crate::domain::criteria::NumericRange {
                min: 200.0,
                max: 300.0,
            },
            crate::domain::criteria::NumericRange { min: 0.5, max: 1.5 },
            crate::domain::criteria::NumericRange { min: 0.0, max: 2.0 },
            1,
        )
        .unwrap();

        let results = BodySearch::new(client)
            .with_limits(100.0, 1, 3)
            .search(&center, &criteria)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Unmapped Body");
        let requests = server.finish().await;
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("POST /api/bodies/search "));
        assert!(requests[1].starts_with("POST /api/bodies/search "));
        let (_, body) = requests[1].split_once("\r\n\r\n").unwrap();
        let payload: Value = serde_json::from_str(body).unwrap();
        assert_eq!(payload["page"], 1);
    }
    #[tokio::test]
    async fn falls_back_to_direct_post_when_recall_reference_has_expired() {
        let first = r#"{"guid":"expired-guid","count":2,"from":0,"results":[{
            "name":"Mapped Body",
            "id64":"1001",
            "system_id64":"2001",
            "system_name":"Nearby",
            "subtype":"Rocky body",
            "distance":1.0,
            "is_landable":true,
            "is_mapped":true,
            "surface_temperature":250,
            "surface_pressure":1.0,
            "gravity":1.0,
            "atmosphere":"Nitrogen"
        }]}"#;
        let second = r#"{"count":2,"from":1,"results":[{
            "name":"Nearby Body",
            "id64":"1002",
            "system_id64":"2002",
            "system_name":"Nearby",
            "subtype":"Rocky body",
            "distance":2.0,
            "is_landable":true,
            "is_mapped":false,
            "surface_temperature":250,
            "surface_pressure":1.0,
            "gravity":1.0,
            "atmosphere":"Nitrogen"
        }]}"#;
        let server = MockServer::start(vec![
            MockResponse::ok(first),
            MockResponse::with_status(reqwest::StatusCode::NOT_FOUND, ""),
            MockResponse::ok(second),
        ])
        .await;

        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let center = ResolvedSystem {
            id64: "42".to_owned(),
            name: "Sol".to_owned(),
            coordinates: super::super::query::Coordinates {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let criteria = SearchCriteria::from_cli(
            "any",
            "any",
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 5000.0,
            },
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 10.0,
            },
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 10.0,
            },
            1,
        )
        .unwrap();

        let results = BodySearch::new(client)
            .with_limits(100.0, 1, 3)
            .search(&center, &criteria)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Nearby Body");
        let requests = server.finish().await;
        assert!(requests[0].starts_with("POST /api/bodies/search "));
        assert!(requests[1].starts_with("GET /api/bodies/search/recall/expired-guid/1 "));
        assert!(requests[2].starts_with("POST /api/bodies/search "));
        let (_, request_body) = requests[2].split_once("\r\n\r\n").unwrap();
        let payload: Value = serde_json::from_str(request_body).unwrap();
        assert_eq!(payload["page"], 1);
    }
    #[tokio::test]
    async fn returns_page_limit_error_instead_of_silent_partial_results() {
        let response_body = r#"{"count":2,"from":0,"results":[{
            "name":"Mapped Body",
            "id64":"1001",
            "system_id64":"2001",
            "system_name":"Nearby",
            "subtype":"Rocky body",
            "distance":1.0,
            "is_landable":true,
            "is_mapped":true,
            "surface_temperature":250,
            "surface_pressure":1.0,
            "gravity":1.0,
            "atmosphere":"Nitrogen"
        }]}"#;
        let server = MockServer::start(vec![MockResponse::ok(response_body)]).await;

        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let center = ResolvedSystem {
            id64: "42".to_owned(),
            name: "Sol".to_owned(),
            coordinates: super::super::query::Coordinates {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let criteria = SearchCriteria::from_cli(
            "any",
            "any",
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 5000.0,
            },
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 10.0,
            },
            crate::domain::criteria::NumericRange {
                min: 0.0,
                max: 10.0,
            },
            1,
        )
        .unwrap();

        let error = BodySearch::new(client)
            .with_limits(100.0, 1, 1)
            .search(&center, &criteria)
            .await
            .unwrap_err();

        assert!(matches!(error, BodySearchError::PageLimit(1)));
        let _ = server.finish().await;
    }
    #[test]
    fn rejects_unknown_page_shape_instead_of_reporting_empty_results() {
        assert!(matches!(
            parse_page(&serde_json::json!({"unexpected": []})),
            Err(BodySearchError::InvalidResponse)
        ));
    }
}
