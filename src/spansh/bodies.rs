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
        let query = BodySearchQuery::new(center.coordinates, self.radius_ly, self.page_size)?;
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

                if body
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

            if page.exhausted {
                completed = true;
                break;
            }

            if page_index + 1 >= self.max_pages {
                return Err(BodySearchError::PageLimit(self.max_pages));
            }

            let Some(search_guid) = guid.as_deref() else {
                // Without a documented cursor/GUID, don't fabricate a next
                // page request. Treat a short page as terminal; a full page
                // is an explicit contract error to avoid silent truncation.
                if page.last_page_size < usize::from(self.page_size) {
                    completed = true;
                    break;
                }
                return Err(BodySearchError::InvalidResponse);
            };

            let next_page = page_index + 1;
            let path = format!("/api/bodies/search/recall/{search_guid}/{next_page}");
            let response = self.client.get_json(&path).await?;
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
    })
}

fn find_guid(value: &Value) -> Option<String> {
    ["guid", "search_guid", "search_id", "id"]
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

    #[test]
    fn rejects_unknown_page_shape_instead_of_reporting_empty_results() {
        assert!(matches!(
            parse_page(&serde_json::json!({"unexpected": []})),
            Err(BodySearchError::InvalidResponse)
        ));
    }
}
