use serde_json::Value;

use super::client::{SpanshClient, SpanshError};
use super::query::Coordinates;

/// Устойчивый идентификатор системы хранится строкой: API может вернуть его
/// числом или строкой, а преобразование через f64 недопустимо для больших ID64.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSystem {
    pub id64: String,
    pub name: String,
    pub coordinates: Coordinates,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error(transparent)]
    Spansh(#[from] SpanshError),
    #[error("не удалось найти систему по имени «{0}»")]
    NotFound(String),
    #[error("ответ Spansh не содержит корректный ID64 или координаты системы")]
    InvalidResponse,
}

/// Разрешает имя системы. Если полное имя не найдено, последовательно
/// отбрасывает уточняющие компоненты справа; из каждого ответа берёт первый
/// подходящий результат в исходном порядке API.
pub struct SystemResolver {
    client: SpanshClient,
}

impl SystemResolver {
    pub fn new(client: SpanshClient) -> Self {
        Self { client }
    }

    pub async fn resolve(&self, input: &str) -> Result<ResolvedSystem, ResolveError> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ResolveError::NotFound(input.to_owned()));
        }

        for end in (1..=parts.len()).rev() {
            let query = parts[..end].join(" ");
            let path = format!(
                "/api/systems/field_values/system_names?q={}",
                encode_query_component(&query)
            );
            let response = self.client.get_json(&path).await?;
            let candidate = if end == parts.len() {
                exact_candidate(&response, &query)
            } else {
                first_candidate(&response)
            };
            if let Some(candidate) = candidate {
                let id64 = candidate.id64.ok_or(ResolveError::InvalidResponse)?;
                if let (Some(name), Some(coordinates)) =
                    (candidate.name.clone(), candidate.coordinates)
                {
                    return Ok(ResolvedSystem {
                        id64,
                        name,
                        coordinates,
                    });
                }

                let detail = self.client.get_json(&format!("/api/system/{id64}")).await?;
                return system_from_detail(&detail, id64, candidate.name)
                    .ok_or(ResolveError::InvalidResponse);
            }
        }

        Err(ResolveError::NotFound(input.to_owned()))
    }
}

#[derive(Debug)]
struct Candidate {
    id64: Option<String>,
    name: Option<String>,
    coordinates: Option<Coordinates>,
}

fn first_candidate(value: &Value) -> Option<Candidate> {
    // Поддерживаем распространённые обёртки API, не фиксируя пока
    // неподтверждённую единственную форму ответа.
    for key in ["systems", "results", "values", "data", "min_max"] {
        if let Some(items) = value.get(key).and_then(Value::as_array)
            && let Some(candidate) = items
                .iter()
                .filter_map(candidate_from_value)
                .find(|candidate| candidate.id64.is_some())
        {
            return Some(candidate);
        }
    }

    if let Some(items) = value.as_array()
        && let Some(candidate) = items
            .iter()
            .filter_map(candidate_from_value)
            .find(|candidate| candidate.id64.is_some())
    {
        return Some(candidate);
    }

    candidate_from_value(value).filter(|candidate| candidate.id64.is_some())
}

fn exact_candidate(value: &Value, expected_name: &str) -> Option<Candidate> {
    let matches_name = |candidate: &Candidate| {
        candidate
            .name
            .as_deref()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(expected_name.trim()))
    };

    for key in ["systems", "results", "values", "data", "min_max"] {
        if let Some(items) = value.get(key).and_then(Value::as_array)
            && let Some(candidate) = items
                .iter()
                .filter_map(candidate_from_value)
                .find(|candidate| matches_name(candidate) && candidate.id64.is_some())
        {
            return Some(candidate);
        }
    }

    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(candidate_from_value)
            .find(|candidate| matches_name(candidate) && candidate.id64.is_some());
    }

    candidate_from_value(value)
        .filter(|candidate| matches_name(candidate) && candidate.id64.is_some())
}

fn candidate_from_value(value: &Value) -> Option<Candidate> {
    if value.is_string() {
        return Some(Candidate {
            id64: None,
            name: value.as_str().map(str::to_owned),
            coordinates: None,
        });
    }

    let id64 = ["id64", "system_id64", "id"]
        .iter()
        .find_map(|key| value.get(key).and_then(id_as_string));
    let name = ["name", "system_name", "value"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str).map(str::to_owned));

    if id64.is_some() || name.is_some() {
        Some(Candidate {
            id64,
            name,
            coordinates: coordinates_from_value(value),
        })
    } else {
        None
    }
}

fn coordinates_from_value(value: &Value) -> Option<Coordinates> {
    let coordinates = value
        .get("coords")
        .or_else(|| value.get("coordinates"))
        .unwrap_or(value);
    let coordinates = Coordinates {
        x: coordinates.get("x").and_then(Value::as_f64)?,
        y: coordinates.get("y").and_then(Value::as_f64)?,
        z: coordinates.get("z").and_then(Value::as_f64)?,
    };
    coordinates.validate().ok()
}

fn system_from_detail(
    detail: &Value,
    fallback_id64: String,
    fallback_name: Option<String>,
) -> Option<ResolvedSystem> {
    let root = detail
        .get("system")
        .or_else(|| detail.get("record"))
        .unwrap_or(detail);
    let id64 = ["id64", "system_id64", "id"]
        .iter()
        .find_map(|key| root.get(key).and_then(id_as_string))
        .unwrap_or(fallback_id64);
    let name = ["name", "system_name"]
        .iter()
        .find_map(|key| root.get(key).and_then(Value::as_str).map(str::to_owned))
        .or(fallback_name)?;

    let coordinate_value = root
        .get("coords")
        .or_else(|| root.get("coordinates"))
        .unwrap_or(root);
    let x = coordinate_value.get("x").and_then(Value::as_f64)?;
    let y = coordinate_value.get("y").and_then(Value::as_f64)?;
    let z = coordinate_value.get("z").and_then(Value::as_f64)?;
    let coordinates = Coordinates { x, y, z }.validate().ok()?;

    Some(ResolvedSystem {
        id64,
        name,
        coordinates,
    })
}

fn id_as_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|id| id.to_string()))
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spansh::test_support::{MockResponse, MockServer};

    #[test]
    fn exact_candidate_selects_only_valid_exact_names_across_response_sections() {
        let cases = [
            (
                "exact name after prefix candidate",
                serde_json::json!({
                    "results": [
                        {"name": "Sol Alpha", "id64": "1"},
                        {"name": "Sol", "id64": "2"}
                    ]
                }),
                "sol",
                Some(("Sol", "2")),
            ),
            (
                "prefix-only results do not become exact matches",
                serde_json::json!({"results": [{"name": "Sol Alpha", "id64": "1"}]}),
                "Sol Beta",
                None,
            ),
            (
                "min_max supplies an exact candidate when results are prefixes",
                serde_json::json!({
                    "results": [{"name": "Thraikuae NS-U e2-18", "id64": "1"}],
                    "values": ["Thraikuae NS-U e2-2"],
                    "min_max": [{
                        "name": "Thraikuae NS-U e2-2",
                        "id64": "2",
                        "x": 10.0,
                        "y": 20.0,
                        "z": 30.0
                    }]
                }),
                "Thraikuae NS-U e2-2",
                Some(("Thraikuae NS-U e2-2", "2")),
            ),
            (
                "matching candidate without id64 is skipped",
                serde_json::json!({
                    "results": [
                        {"name": "Sol"},
                        {"name": "Sol", "id64": "42"}
                    ]
                }),
                "Sol",
                Some(("Sol", "42")),
            ),
        ];

        for (case, response, requested, expected) in cases {
            let candidate = exact_candidate(&response, requested);
            match expected {
                Some((name, id64)) => {
                    let candidate =
                        candidate.unwrap_or_else(|| panic!("{case}: no exact candidate"));
                    assert_eq!(candidate.name.as_deref(), Some(name), "{case}");
                    assert_eq!(candidate.id64.as_deref(), Some(id64), "{case}");
                }
                None => assert!(candidate.is_none(), "{case}"),
            }
        }
    }

    #[test]
    fn first_candidate_parses_supported_shapes_and_skips_name_only_entries() {
        let cases = [
            (
                "name-only candidate is skipped",
                serde_json::json!({
                    "results": [
                        "Col 285 Sector BQ-Y",
                        {"name": "Col 285 Sector BQ-Y", "id64": "9223372036854775807"}
                    ]
                }),
                "Col 285 Sector BQ-Y",
                "9223372036854775807",
                None,
            ),
            (
                "flat coordinates are preserved",
                serde_json::json!({
                    "results": [{
                        "name": "Sol",
                        "id64": "10477373803",
                        "x": 0.0,
                        "y": 0.0,
                        "z": 0.0
                    }]
                }),
                "Sol",
                "10477373803",
                Some(Coordinates {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }),
            ),
            (
                "large string id64 is not rounded",
                serde_json::json!({
                    "results": [{
                        "name": "Sol",
                        "id64": "9223372036854775807"
                    }]
                }),
                "Sol",
                "9223372036854775807",
                None,
            ),
            (
                "numeric u64 id64 is normalized to a string",
                serde_json::json!([{
                    "system_name": "Sol",
                    "system_id64": 123456789_u64
                }]),
                "Sol",
                "123456789",
                None,
            ),
        ];

        for (case, response, name, id64, coordinates) in cases {
            let candidate = first_candidate(&response)
                .unwrap_or_else(|| panic!("{case}: no candidate returned"));
            assert_eq!(candidate.name.as_deref(), Some(name), "{case}");
            assert_eq!(candidate.id64.as_deref(), Some(id64), "{case}");
            assert_eq!(candidate.coordinates, coordinates, "{case}");
        }
    }

    #[test]
    fn detail_parser_accepts_supported_spansh_record_shapes() {
        let coordinates = Coordinates {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let cases = [
            (
                "record envelope",
                serde_json::json!({
                    "record": {
                        "name": "Sol",
                        "id64": "42",
                        "x": 1.0,
                        "y": 2.0,
                        "z": 3.0
                    }
                }),
            ),
            (
                "nested system coordinates",
                serde_json::json!({
                    "system": {
                        "name": "Sol",
                        "id64": "42",
                        "coords": {"x": 1.0, "y": 2.0, "z": 3.0}
                    }
                }),
            ),
        ];

        for (case, detail) in cases {
            let system = system_from_detail(&detail, "0".to_owned(), None)
                .unwrap_or_else(|| panic!("{case}: detail could not be parsed"));
            assert_eq!(system.name, "Sol", "{case}");
            assert_eq!(system.id64, "42", "{case}");
            assert_eq!(system.coordinates, coordinates, "{case}");
        }
    }

    #[tokio::test]
    async fn resolver_drops_trailing_components_and_loads_system_details() {
        let responses = [
            r#"{"results":[]}"#,
            r#"{"results":[{"name":"Col 285 Sector BQ-Y","id64":"9223372036854775807"}]}"#,
            r#"{"name":"Col 285 Sector BQ-Y","id64":"9223372036854775807","coords":{"x":10.0,"y":20.0,"z":30.0}}"#,
        ];
        let server = MockServer::start(responses.into_iter().map(MockResponse::ok).collect()).await;

        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let resolver = SystemResolver::new(client);
        let system = resolver.resolve("Col 285 Sector BQ-Y d40").await.unwrap();

        assert_eq!(system.name, "Col 285 Sector BQ-Y");
        assert_eq!(system.id64, "9223372036854775807");
        assert_eq!(
            system.coordinates,
            Coordinates {
                x: 10.0,
                y: 20.0,
                z: 30.0
            }
        );

        let requests = server.finish().await;
        assert!(requests[0].starts_with(
            "GET /api/systems/field_values/system_names?q=Col%20285%20Sector%20BQ-Y%20d40 "
        ));
        assert!(requests[1].starts_with(
            "GET /api/systems/field_values/system_names?q=Col%20285%20Sector%20BQ-Y "
        ));
        assert!(requests[2].starts_with("GET /api/system/9223372036854775807 "));
    }

    #[tokio::test]
    async fn resolver_uses_coordinates_from_search_without_detail_request() {
        let body = r#"{"count":1,"query":"Sol","results":[{"name":"Sol","id64":"42","x":1.0,"y":2.0,"z":3.0}]}"#;
        let server = MockServer::start(vec![MockResponse::ok(body)]).await;
        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let system = SystemResolver::new(client).resolve("Sol").await.unwrap();

        assert_eq!(system.id64, "42");
        assert_eq!(system.name, "Sol");
        assert_eq!(
            system.coordinates,
            Coordinates {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
        let requests = server.finish().await;
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /api/systems/field_values/system_names?q=Sol "));
    }

    #[tokio::test]
    async fn exact_system_name_wins_over_earlier_prefix_candidate() {
        let responses = [
            r#"{"results":[{"name":"Sol Alpha","id64":"1"},{"name":"Sol","id64":"2"}]}"#,
            r#"{"name":"Sol","id64":"2","coords":{"x":1.0,"y":2.0,"z":3.0}}"#,
        ];
        let server = MockServer::start(responses.into_iter().map(MockResponse::ok).collect()).await;
        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let system = SystemResolver::new(client).resolve("Sol").await.unwrap();

        assert_eq!(system.name, "Sol");
        assert_eq!(system.id64, "2");
        let requests = server.finish().await;
        assert!(requests[1].starts_with("GET /api/system/2 "));
    }
}
