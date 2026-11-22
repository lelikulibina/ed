use reqwest::StatusCode;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

pub(crate) struct MockResponse {
    status: StatusCode,
    body: String,
}

impl MockResponse {
    pub(crate) fn ok(body: impl Into<String>) -> Self {
        Self::with_status(StatusCode::OK, body)
    }

    pub(crate) fn with_status(status: StatusCode, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }
}

pub(crate) struct MockServer {
    base_url: String,
    task: JoinHandle<Vec<String>>,
}

impl MockServer {
    pub(crate) async fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock server must bind an ephemeral local port");
        let address = listener
            .local_addr()
            .expect("mock server must expose its local address");
        let task = tokio::spawn(async move {
            let mut requests = Vec::with_capacity(responses.len());
            for response in responses {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("mock server must accept the client request");
                requests.push(read_request(&mut socket).await);

                let reason = response.status.canonical_reason().unwrap_or("Unknown");
                let raw_response = format!(
                    "HTTP/1.1 {} {}\\r\\ncontent-type: application/json\\r\\ncontent-length: {}\\r\\nconnection: close\\r\\n\\r\\n{}",
                    response.status.as_u16(),
                    reason,
                    response.body.len(),
                    response.body
                );
                socket
                    .write_all(raw_response.as_bytes())
                    .await
                    .expect("mock server must write the queued response");
            }
            requests
        });

        Self {
            base_url: format!("http://{address}"),
            task,
        }
    }

    pub(crate) fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub(crate) async fn finish(self) -> Vec<String> {
        self.task
            .await
            .expect("mock server task must finish without panicking")
    }
}

async fn read_request(socket: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        let read = socket
            .read(&mut chunk)
            .await
            .expect("mock server must read the request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);

        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\\r\\n\\r\\n") {
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

    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{MockResponse, MockServer};
    use crate::spansh::SpanshClient;

    #[tokio::test]
    async fn mock_server_captures_request_and_serves_queued_json() {
        let server = MockServer::start(vec![MockResponse::ok(r#"{"ok":true}"#)]).await;
        let client = SpanshClient::new(server.base_url(), 1).unwrap();

        let response = client.get_json("/fixture").await.unwrap();
        assert_eq!(response["ok"], true);

        let requests = server.finish().await;
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /fixture "));
    }
}
