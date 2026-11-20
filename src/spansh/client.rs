use std::{sync::Arc, time::Duration};

use tokio::time::Instant;

use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::{
    sync::{Mutex, Semaphore},
    time::sleep,
};

/// Низкоуровневый HTTP-транспорт Spansh. Контракт конкретных endpoint-ов
/// намеренно остаётся в query/resolver слоях: API Spansh документирован не полностью.
#[derive(Clone)]
pub struct SpanshClient {
    http: Client,
    base_url: Arc<str>,
    permits: Arc<Semaphore>,
    max_attempts: usize,
    retry_delay: Duration,
    request_interval: Duration,
    last_request_at: Arc<Mutex<Option<Instant>>>,
}

impl SpanshClient {
    pub fn new(base_url: impl Into<String>, concurrency: usize) -> Result<Self, reqwest::Error> {
        let http = Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            base_url: Arc::from(base_url.into().trim_end_matches('/').to_owned()),
            permits: Arc::new(Semaphore::new(concurrency.max(1))),
            max_attempts: 4,
            retry_delay: Duration::from_millis(500),
            request_interval: Duration::from_secs(1),
            last_request_at: Arc::new(Mutex::new(None)),
        })
    }

    pub fn spansh(concurrency: usize) -> Result<Self, reqwest::Error> {
        Self::new("https://spansh.co.uk", concurrency)
    }

    pub async fn get_json(&self, path: &str) -> Result<Value, SpanshError> {
        self.request_json(Method::Get, path, None).await
    }

    pub async fn post_json(&self, path: &str, body: &Value) -> Result<Value, SpanshError> {
        self.request_json(Method::Post, path, Some(body)).await
    }

    async fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, SpanshError> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| SpanshError::Closed)?;
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        for attempt in 0..self.max_attempts {
            self.wait_for_request_slot().await;
            let mut request = match method {
                Method::Get => self.http.get(&url),
                Method::Post => self.http.post(&url),
            };
            if let Some(json) = body {
                request = request.json(json);
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    return response.json::<Value>().await.map_err(SpanshError::Http);
                }
                Ok(response) => {
                    let status = response.status();
                    let retryable =
                        status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                    let message = response.text().await.unwrap_or_default();
                    if !retryable || attempt + 1 == self.max_attempts {
                        return Err(SpanshError::Status { status, message });
                    }
                }
                Err(error) => {
                    if !error.is_timeout() && !error.is_connect() {
                        return Err(SpanshError::Http(error));
                    }
                    if attempt + 1 == self.max_attempts {
                        return Err(SpanshError::Http(error));
                    }
                }
            }

            sleep(self.retry_delay.saturating_mul(1u32 << attempt.min(8))).await;
        }

        unreachable!("retry loop always returns on the final attempt")
    }

    async fn wait_for_request_slot(&self) {
        let mut last_request_at = self.last_request_at.lock().await;
        if let Some(last_request_at) = *last_request_at {
            let elapsed = last_request_at.elapsed();
            if elapsed < self.request_interval {
                sleep(self.request_interval - elapsed).await;
            }
        }
        *last_request_at = Some(Instant::now());
    }
}

#[derive(Clone, Copy)]
enum Method {
    Get,
    Post,
}

#[derive(Debug, thiserror::Error)]
pub enum SpanshError {
    #[error("HTTP error from Spansh: {0}")]
    Http(#[source] reqwest::Error),
    #[error("Spansh returned HTTP {status}: {message}")]
    Status { status: StatusCode, message: String },
    #[error("Spansh client is shutting down")]
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn respond(listener: TcpListener, responses: Vec<&'static str>) {
        for response in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = socket.read(&mut request).await.unwrap();
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn get_json_decodes_response_from_injected_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(respond(
            listener,
            vec![
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 11\r\nconnection: close\r\n\r\n{\"ok\":true}",
            ],
        ));

        let client = SpanshClient::new(format!("http://{address}"), 1).unwrap();
        let value = client.get_json("/fixture").await.unwrap();
        assert!(value["ok"].as_bool().unwrap());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn enforces_configured_minimum_interval_between_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 11\r\nconnection: close\r\n\r\n{\"ok\":true}";
        let server = tokio::spawn(respond(listener, vec![response, response]));

        let mut client = SpanshClient::new(format!("http://{address}"), 1).unwrap();
        client.request_interval = Duration::from_millis(25);
        let started = std::time::Instant::now();
        client.get_json("/first").await.unwrap();
        client.get_json("/second").await.unwrap();

        assert!(started.elapsed() >= Duration::from_millis(20));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn retries_server_errors_but_returns_successful_json() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(respond(
            listener,
            vec![
                "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 11\r\nconnection: close\r\n\r\n{\"ok\":true}",
            ],
        ));

        let mut client = SpanshClient::new(format!("http://{address}"), 1).unwrap();
        client.retry_delay = Duration::from_millis(1);
        client.request_interval = Duration::from_millis(1);
        let value = client.get_json("/fixture").await.unwrap();
        assert!(value["ok"].as_bool().unwrap());
        server.await.unwrap();
    }
}
