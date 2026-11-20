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
        let initial = self.client.post_json("/api/bodies/search", &query.to_payload()).await?;

        let mut page = parse_page(&initial)?;
        let mut guid = find_guid(&initial);
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        let mut completed = false;

        for page_index in 0..self.max_pages {
            for body in page.bodies.drain(..) {
                let identity = body.id64.clone().unwrap_or_else(|| {
                    format!("{}:{}", body.system_id64.as_deref().unwrap_or(""), body.name)
                });
                if !seen.insert(identity) {
                    continue;
                }

                if criteria.matches(&body) {
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
            value.get("search")
                .and_then(|search| ["guid", "id"].iter().find_map(|key| {
                    search.get(key).and_then(Value::as_str).map(str::to_owned)
                }))
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

    #[test]
    fn rejects_unknown_page_shape_instead_of_reporting_empty_results() {
        assert!(matches!(
            parse_page(&serde_json::json!({"unexpected": []})),
            Err(BodySearchError::InvalidResponse)
        ));
    }
}
