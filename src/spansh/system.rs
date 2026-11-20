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
            let path = format!("/api/search/systems?q={}", encode_query_component(&query));
            let response = self.client.get_json(&path).await?;
            let candidate = if end == parts.len() {
                exact_candidate(&response, &query)
            } else {
                first_candidate(&response)
            };
            if let Some(candidate) = candidate {
                let id64 = candidate.id64.ok_or(ResolveError::InvalidResponse)?;
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
}

fn first_candidate(value: &Value) -> Option<Candidate> {
    // Поддерживаем распространённые обёртки API, не фиксируя пока
    // неподтверждённую единственную форму ответа.
    for key in ["systems", "results", "values", "data"] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            for item in items {
                if let Some(candidate) = candidate_from_value(item) {
                    return Some(candidate);
                }
            }
        }
    }

    if let Some(items) = value.as_array() {
        for item in items {
            if let Some(candidate) = candidate_from_value(item) {
                return Some(candidate);
            }
        }
    }

    candidate_from_value(value)
}

fn exact_candidate(value: &Value, expected_name: &str) -> Option<Candidate> {
    let matches_name = |candidate: &Candidate| {
        candidate
            .name
            .as_deref()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(expected_name.trim()))
    };

    for key in ["systems", "results", "values", "data"] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            return items
                .iter()
                .filter_map(candidate_from_value)
                .find(matches_name);
        }
    }

    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(candidate_from_value)
            .find(matches_name);
    }

    candidate_from_value(value).filter(matches_name)
}

fn candidate_from_value(value: &Value) -> Option<Candidate> {
    if value.is_string() {
        return Some(Candidate {
            id64: None,
            name: value.as_str().map(str::to_owned),
        });
    }

    let id64 = ["id64", "system_id64", "id"]
        .iter()
        .find_map(|key| value.get(key).and_then(id_as_string));
    let name = ["name", "system_name", "value"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str).map(str::to_owned));

    if id64.is_some() || name.is_some() {
        Some(Candidate { id64, name })
    } else {
        None
    }
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

    #[test]
    fn exact_candidate_skips_prefix_matches_and_returns_none_without_exact_name() {
        let value = serde_json::json!({
            "results": [
                {"name": "Sol Alpha", "id64": "1"},
                {"name": "Sol", "id64": "2"}
            ]
        });

        let exact = exact_candidate(&value, "sol").unwrap();
        assert_eq!(exact.name.as_deref(), Some("Sol"));
        assert_eq!(exact.id64.as_deref(), Some("2"));
        assert!(exact_candidate(&value, "Sol Beta").is_none());
    }

    #[test]
    fn candidate_parser_preserves_large_id64_as_string() {
        let value = serde_json::json!({
            "results": [{
                "name": "Sol",
                "id64": "9223372036854775807"
            }]
        });
        let candidate = first_candidate(&value).unwrap();
        assert_eq!(candidate.id64.as_deref(), Some("9223372036854775807"));
        assert_eq!(candidate.name.as_deref(), Some("Sol"));
    }

    #[test]
    fn candidate_parser_accepts_numeric_u64_id64() {
        let value = serde_json::json!([{
            "system_name": "Sol",
            "system_id64": 123456789_u64
        }]);
        let candidate = first_candidate(&value).unwrap();
        assert_eq!(candidate.id64.as_deref(), Some("123456789"));
    }

    #[test]
    fn detail_parser_accepts_spansh_record_envelope() {
        let detail = serde_json::json!({
            "record": {
                "name": "Sol",
                "id64": "42",
                "x": 1.0,
                "y": 2.0,
                "z": 3.0
            }
        });

        let system = system_from_detail(&detail, "0".to_owned(), None).unwrap();
        assert_eq!(system.name, "Sol");
        assert_eq!(system.id64, "42");
        assert_eq!(
            system.coordinates,
            Coordinates {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
    }

    #[test]
    fn detail_parser_accepts_nested_coords() {
        let detail = serde_json::json!({
            "system": {
                "name": "Sol",
                "id64": "42",
                "coords": {"x": 1.0, "y": 2.0, "z": 3.0}
            }
        });
        let system = system_from_detail(&detail, "0".to_owned(), None).unwrap();
        assert_eq!(system.name, "Sol");
        assert_eq!(system.id64, "42");
        assert_eq!(
            system.coordinates,
            Coordinates {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
    }

    #[tokio::test]
    async fn resolver_drops_trailing_components_and_loads_system_details() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responses = [
            r#"{"results":[]}"#,
            r#"{"results":[{"name":"Col 285 Sector BQ-Y","id64":"9223372036854775807"}]}"#,
            r#"{"name":"Col 285 Sector BQ-Y","id64":"9223372036854775807","coords":{"x":10.0,"y":20.0,"z":30.0}}"#,
        ];
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 4096];
                let size = socket.read(&mut request).await.unwrap();
                requests.push(String::from_utf8_lossy(&request[..size]).to_string());
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

        let requests = server.await.unwrap();
        assert!(
            requests[0].starts_with("GET /api/search/systems?q=Col%20285%20Sector%20BQ-Y%20d40 ")
        );
        assert!(requests[1].starts_with("GET /api/search/systems?q=Col%20285%20Sector%20BQ-Y "));
        assert!(requests[2].starts_with("GET /api/system/9223372036854775807 "));
    }

    #[tokio::test]
    async fn exact_system_name_wins_over_earlier_prefix_candidate() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responses = [
            r#"{"results":[{"name":"Sol Alpha","id64":"1"},{"name":"Sol","id64":"2"}]}"#,
            r#"{"name":"Sol","id64":"2","coords":{"x":1.0,"y":2.0,"z":3.0}}"#,
        ];
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 2048];
                let size = socket.read(&mut request).await.unwrap();
                requests.push(String::from_utf8_lossy(&request[..size]).to_string());
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
        let system = SystemResolver::new(client).resolve("Sol").await.unwrap();

        assert_eq!(system.name, "Sol");
        assert_eq!(system.id64, "2");
        let requests = server.await.unwrap();
        assert!(requests[1].starts_with("GET /api/system/2 "));
    }

    #[test]
    fn query_component_is_percent_encoded() {
        assert_eq!(
            encode_query_component("Col 285 Sector BQ-Y d40"),
            "Col%20285%20Sector%20BQ-Y%20d40"
        );
    }
}
