use crate::{
    domain::{body::Body, criteria::SearchCriteria},
    spansh::{BodySearch, BodySearchError, ResolvedSystem, SpanshClient, SystemResolver},
};

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error(transparent)]
    Resolve(#[from] crate::spansh::ResolveError),
    #[error(transparent)]
    Bodies(#[from] BodySearchError),
}

pub struct SearchUseCase {
    client: SpanshClient,
}

impl SearchUseCase {
    pub fn new(client: SpanshClient) -> Self {
        Self { client }
    }

    pub async fn resolve_center(&self, system_name: &str) -> Result<ResolvedSystem, SearchError> {
        SystemResolver::new(self.client.clone())
            .resolve(system_name)
            .await
            .map_err(Into::into)
    }

    pub async fn execute(
        &self,
        center: &ResolvedSystem,
        criteria: &SearchCriteria,
    ) -> Result<Vec<Body>, SearchError> {
        BodySearch::new(self.client.clone())
            .search(center, criteria)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::criteria::NumericRange;
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
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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

    #[tokio::test]
    async fn resolves_center_searches_bodies_and_returns_filtered_results() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responses = [
            r#"{"results":[{"name":"Sol","id64":"42"}]}"#,
            r#"{"name":"Sol","id64":"42","coords":{"x":1.0,"y":2.0,"z":3.0}}"#,
            r#"{"has_more":false,"results":[{
                "name":"Nearby 1",
                "id64":"1001",
                "system":{"id64":"2001","name":"Nearby System"},
                "subtype":"Rocky body",
                "is_landable":true,
                "is_mapped":false,
                "surface_temperature":250,
                "surface_pressure":1.0,
                "gravity":1.0,
                "atmosphere":"Nitrogen",
                "distance":12.0
            }]}"#,
        ];
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in responses {
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
        let search = SearchUseCase::new(client);
        let center = search.resolve_center("Sol").await.unwrap();
        let criteria = SearchCriteria::from_cli(
            "rocky body",
            "nitrogen",
            NumericRange {
                min: 200.0,
                max: 300.0,
            },
            NumericRange { min: 0.5, max: 1.5 },
            NumericRange { min: 0.0, max: 2.0 },
            1,
        )
        .unwrap();

        let results = search.execute(&center, &criteria).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Nearby 1");
        assert_eq!(results[0].system_name.as_deref(), Some("Nearby System"));

        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("GET /api/search/systems?q=Sol "));
        assert!(requests[1].starts_with("GET /api/system/42 "));
        assert!(requests[2].starts_with("POST /api/bodies/search "));
    }
}
