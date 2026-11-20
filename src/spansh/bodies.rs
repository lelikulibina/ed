use std::collections::HashSet;

use serde_json::Value;

use crate::domain::{body::Body, criteria::SearchCriteria};

use super::{
    client::{SpanshClient, SpanshError},
    query::{BodySearchQuery, QueryError},
    system::ResolvedSystem,
};

const DEFAULT_RADIUS_LY: f64 = 100.0;
const PAGE_SIZE: u16 = 100;
const MAX_PAGES: u32 = 100;

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

    /// Поиск тел вокруг центральной системы. Физические фильтры применяются
    /// локально, поскольку точный список фильтров Spansh Search API ещё
    /// не подтверждён живым контрактным запросом.
    pub async fn search(
        &self,
        center: &ResolvedSystem,
        criteria: &SearchCriteria,
    ) -> Result<Vec<Body>, BodySearchError> {
        let mut query = BodySearchQuery::new(center.coordinates, self.radius_ly, self.page_size)?;
        let initial = self
            .client
            .post_json("/api/bodies/search", &query.to_payload())
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
                self.client.get_json(&path).await?
            } else {
                // Live API probe confirms page=N advances the result set; use it
                // as fallback when no search_reference is supplied.
                query.page = next_page;
                self.client
                    .post_json("/api/bodies/search", &query.to_payload())
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
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 2048];
            loop {
                let read = socket.read(&mut chunk).await.unwrap();
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if bytes.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            String::from_utf8_lossy(&bytes).to_string()
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let first = include_str!("../../tests/fixtures/spansh/body-search-page-0.json").to_owned();
        let second = include_str!("../../tests/fixtures/spansh/body-search-page-1.json").to_owned();

        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in [first, second] {
                let (mut socket, _) = listener.accept().await.unwrap();
                requests.push(read_request(&mut socket).await);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });

        let client = SpanshClient::new(format!("http://{address}"), 1).unwrap();
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
        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("POST /api/bodies/search "));
        let (_, request_body) = requests[0].split_once("\r\n\r\n").unwrap();
        let payload: Value = serde_json::from_str(request_body).unwrap();
        assert_eq!(payload["reference_coords"]["x"], 1.0);
        assert_eq!(payload["reference_coords"]["y"], 2.0);
        assert_eq!(payload["reference_coords"]["z"], 3.0);
        assert!(requests[1].starts_with("GET /api/bodies/search/recall/test-guid/1 "));
    }

    #[tokio::test]
    async fn search_uses_post_page_fallback_when_reference_is_missing() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responses = [
            r#"{"count":2,"from":0,"size":1,"results":[{
                "name":"Mapped Body",
                "id64":"1001",
                "system_id64":"2001",
                "system_name":"Nearby A",
                "subtype":"Rocky body",
                "is_landable":true,
                "is_mapped":true,
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

        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut chunk = [0_u8; 2048];
                loop {
                    let read = socket.read(&mut chunk).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..read]);
                    if let Some(header_end) =
                        bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let headers = String::from_utf8_lossy(&bytes[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if bytes.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8_lossy(&bytes).to_string());
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });

        let client = SpanshClient::new(format!("http://{address}"), 1).unwrap();
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
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("POST /api/bodies/search "));
        assert!(requests[1].starts_with("POST /api/bodies/search "));
        let (_, body) = requests[1].split_once("\r\n\r\n").unwrap();
        let payload: Value = serde_json::from_str(body).unwrap();
        assert_eq!(payload["page"], 1);
    }

    #[tokio::test]
    #[ignore = "requires live network; low-volume Spansh contract and pagination probe"]
    async fn live_spansh_body_search_contract() {
        let client = SpanshClient::spansh(1).unwrap();
        let system_search = client
            .get_json("/api/search/systems?q=Sol")
            .await
            .expect("system search endpoint must respond");
        let search_keys = system_search
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|| "non-object response".to_owned());
        let search_preview = ["systems", "results", "values", "data"]
            .iter()
            .find_map(|key| {
                system_search
                    .get(key)
                    .and_then(Value::as_array)
                    .map(|items| items.iter().take(3).cloned().collect::<Vec<_>>())
            })
            .or_else(|| {
                system_search
                    .as_array()
                    .map(|items| items.iter().take(3).cloned().collect::<Vec<_>>())
            });
        eprintln!("Live system search: keys=[{search_keys}], preview={search_preview:?}");

        let system_detail = client
            .get_json("/api/system/10477373803")
            .await
            .expect("system detail endpoint must respond");
        let system_record = system_detail.get("record").unwrap_or(&system_detail);
        eprintln!(
            "Live system detail: name={:?}, id64={:?}, coords=({:?}, {:?}, {:?})",
            system_record.get("name"),
            system_record.get("id64"),
            system_record.get("x"),
            system_record.get("y"),
            system_record.get("z")
        );

        let center = super::super::system::SystemResolver::new(client.clone())
            .resolve("Sol")
            .await
            .expect("Spansh system resolver must find Sol");
        assert_eq!(center.name, "Sol");
        eprintln!(
            "Resolved live center: {} ID64={} coords=({}, {}, {})",
            center.name,
            center.id64,
            center.coordinates.x,
            center.coordinates.y,
            center.coordinates.z
        );

        let mut query = BodySearchQuery::new(center.coordinates, 1.0, 1).unwrap();

        let response = client
            .post_json("/api/bodies/search", &query.to_payload())
            .await
            .expect("Spansh body-search POST must succeed");
        let page = parse_page(&response)
            .expect("live response must match one of the supported page envelopes");

        let response_keys = response
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|| "non-object response".to_owned());
        let first_body = page.bodies.first();
        eprintln!(
            "Spansh page 0: keys=[{response_keys}], count={:?}, from={:?}, size={:?}, search={:?}, search_reference={:?}, rows={}, guid={:?}, first body={:?}",
            response.get("count"),
            response.get("from"),
            response.get("size"),
            response.get("search"),
            response.get("search_reference"),
            page.bodies.len(),
            find_guid(&response),
            first_body.map(|body| (
                &body.name,
                &body.id64,
                &body.system_id64,
                &body.system_name,
                &body.subtype,
                body.distance_ly,
                body.is_landable,
                body.is_mapped,
                body.surface_temperature,
                body.surface_pressure,
                body.gravity,
                &body.atmosphere
            ))
        );

        if let Some(guid) = find_guid(&response) {
            let recall = client
                .get_json(&format!("/api/bodies/search/recall/{guid}/1"))
                .await
                .expect("recall page request must succeed");
            let next =
                parse_page(&recall).expect("recall response must match a supported page envelope");
            eprintln!(
                "Spansh recall page 1: {} rows, first body={:?}",
                next.bodies.len(),
                next.bodies.first().map(|body| (
                    &body.name,
                    &body.id64,
                    &body.system_id64,
                    &body.system_name,
                    &body.subtype,
                    body.distance_ly,
                    body.is_landable,
                    body.is_mapped,
                    body.surface_temperature,
                    body.surface_pressure,
                    body.gravity,
                    &body.atmosphere
                ))
            );

            let last_response = client
                .get_json(&format!("/api/bodies/search/recall/{guid}/39"))
                .await
                .expect("last recall page request must succeed");
            let last_page = parse_page(&last_response)
                .expect("last recall page must match a supported page envelope");
            assert_eq!(last_page.from, Some(39));
            assert!(is_last_page(&last_page, 39, 1));
            eprintln!(
                "Spansh recall page 39: count={:?}, from={:?}, rows={}, exhausted={}",
                last_page.total_count,
                last_page.from,
                last_page.last_page_size,
                is_last_page(&last_page, 39, 1)
            );

            let mut wide_query = BodySearchQuery::new(center.coordinates, 100.0, 1).unwrap();
            let wide_response = client
                .post_json("/api/bodies/search", &wide_query.to_payload())
                .await
                .expect("100-ly body-search POST must succeed");
            let wide_page = parse_page(&wide_response)
                .expect("100-ly response must match a supported page envelope");
            let wide_count = wide_page.total_count.unwrap_or(wide_page.last_page_size);
            eprintln!("Spansh 100-ly search: count={wide_count}");

            assert!(
                wide_count > 40,
                "100-ly search should include bodies beyond Sol's 40-body system"
            );
            let neighbor_response = if let Some(reference) = find_guid(&wide_response) {
                client
                    .get_json(&format!("/api/bodies/search/recall/{reference}/40"))
                    .await
                    .expect("100-ly recall page 40 must succeed")
            } else {
                wide_query.page = 40;
                client
                    .post_json("/api/bodies/search", &wide_query.to_payload())
                    .await
                    .expect("100-ly page=40 POST must succeed")
            };
            let neighbor_page = parse_page(&neighbor_response)
                .expect("100-ly page 40 must match a supported page envelope");
            let neighbor = neighbor_page
                .bodies
                .first()
                .expect("page 40 should contain a body from another system");
            assert_ne!(
                neighbor.system_id64.as_deref(),
                Some(center.id64.as_str()),
                "page 40 must move beyond the central system"
            );
            assert!(
                neighbor.distance_ly.is_some_and(|distance| distance > 0.0),
                "neighboring-system body should have positive distance from Sol"
            );
            eprintln!(
                "Spansh 100-ly page 40: system={:?}, ID64={:?}, distance={:?}",
                neighbor.system_name, neighbor.system_id64, neighbor.distance_ly
            );

            let bulk_query = BodySearchQuery::new(center.coordinates, 100.0, 100).unwrap();
            let bulk_response = client
                .post_json("/api/bodies/search", &bulk_query.to_payload())
                .await
                .expect("100-ly page-size-100 POST must succeed");
            let bulk_page = parse_page(&bulk_response)
                .expect("100-ly page-size-100 response must parse");
            let total_count = bulk_page
                .total_count
                .expect("live response must include total count");
            assert_eq!(bulk_page.from, Some(0));
            assert_eq!(bulk_page.last_page_size, 100);
            assert!(
                total_count <= 10_000,
                "current 100-page safety limit must cover the provider result cap"
            );

            let last_index = u32::try_from((total_count - 1) / 100).unwrap();
            let last_response = if let Some(reference) = find_guid(&bulk_response) {
                client
                    .get_json(&format!(
                        "/api/bodies/search/recall/{reference}/{last_index}"
                    ))
                    .await
                    .expect("100-row last recall page must succeed")
            } else {
                let mut last_query = bulk_query;
                last_query.page = last_index;
                client
                    .post_json("/api/bodies/search", &last_query.to_payload())
                    .await
                    .expect("100-row last POST page must succeed")
            };
            let bulk_last_page = parse_page(&last_response)
                .expect("100-row last page response must parse");
            assert_eq!(
                bulk_last_page.from,
                Some(last_index as usize * 100)
            );
            assert!(is_last_page(&bulk_last_page, last_index, 100));
            eprintln!(
                "Spansh 100-row last page: total={total_count}, index={last_index}, from={:?}, rows={}",
                bulk_last_page.from,
                bulk_last_page.last_page_size
            );
        } else {
            query.page = 1;
            let response = client
                .post_json("/api/bodies/search", &query.to_payload())
                .await
                .expect("page=1 body-search POST must succeed when no recall reference exists");
            let next = parse_page(&response)
                .expect("page=1 response must match a supported page envelope");
            let next_id = next.bodies.first().and_then(|body| body.id64.as_deref());
            let first_id = page.bodies.first().and_then(|body| body.id64.as_deref());
            eprintln!(
                "Spansh page=1: count={:?}, from={:?}, size={:?}, {} rows, first body ID64={next_id:?}",
                response.get("count"),
                response.get("from"),
                response.get("size"),
                next.bodies.len()
            );
            if let (Some(first_id), Some(next_id)) = (first_id, next_id) {
                assert_ne!(first_id, next_id, "page=1 must advance beyond page=0");
            }
        }
    }

    #[test]
    fn rejects_unknown_page_shape_instead_of_reporting_empty_results() {
        assert!(matches!(
            parse_page(&serde_json::json!({"unexpected": []})),
            Err(BodySearchError::InvalidResponse)
        ));
    }
}
