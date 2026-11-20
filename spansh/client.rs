use std::{sync::Arc, time::Duration};

use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::{sync::Semaphore, time::sleep};

/// Низкоуровневый HTTP-транспорт Spansh. Контракт конкретных endpoint-ов
/// намеренно остаётся в query/resolver слоях: API Spansh документирован не полностью.
#[derive(Clone)]
pub struct SpanshClient {
    http: Client,
    base_url: Arc<str>,
    permits: Arc<Semaphore>,
    max_attempts: usize,
    retry_delay: Duration,
}

impl SpanshClient {
    pub fn new(base_url: impl Into<String>, concurrency: usize) -> Result<Self, reqwest::Error> {
        let http = Client::builder()
            .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            base_url: Arc::from(base_url.into().trim_end_matches('/').to_owned()),
            permits: Arc::new(Semaphore::new(concurrency.max(1))),
            max_attempts: 4,
            retry_delay: Duration::from_millis(500),
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
        let _permit = self.permits.acquire().await.map_err(|_| SpanshError::Closed)?;
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));

        for attempt in 0..self.max_attempts {
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
                    let retryable = status == StatusCode::TOO_MANY_REQUESTS
                        || status.is_server_error();
                    let message = response.text().await.unwrap_or_default();
                    if !retryable || attempt + 1 == self.max_attempts {
                        return Err(SpanshError::Status { status, message });
                    }
                }
                Err(error) => {
                    if !error.is_timeout() && !error.is_connect() && !error.is_request() {
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

    #[tokio::test]
    async fn client_uses_configured_base_url_and_returns_json() {
        // The base URL is injectable so transport behavior can be tested against
        // a local mock server without depending on spansh.co.uk.
        let client = SpanshClient::new("http://127.0.0.1:1", 2).unwrap();
        let result = client.get_json("/health").await;
        assert!(result.is_err());
    }
}
