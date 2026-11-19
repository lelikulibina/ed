use anyhow::{Context, Result, bail};
use reqwest::{Client, header};
use scraper::Html;
use std::time::Duration;

/// Основной парсер с единым HTTP-клиентом.
#[derive(Clone)]
pub struct ParserCore {
    pub client: Client,
}

impl ParserCore {
    pub fn new() -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("text/html,application/xhtml+xml"),
        );

        let client = Client::builder()
            .user_agent(concat!("ed-planet-search/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("Не удалось создать HTTP-клиент")?;

        Ok(Self { client })
    }

    /// Загружает JSON-ответ EDSM API с проверкой статуса и формата.
    pub async fn fetch_json(&self, url: &str) -> Result<serde_json::Value> {
        let response = self
            .client
            .get(url)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .with_context(|| format!("Ошибка API-запроса к {url}"))?;

        let status = response.status();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_ascii_lowercase);

        let body = response
            .text()
            .await
            .with_context(|| format!("Не удалось прочитать API-ответ от {final_url}"))?;

        if looks_like_cloudflare_challenge(&body) {
            bail!(
                "EDSM API вернул проверку Cloudflare вместо JSON ({final_url}, HTTP {status}). Автоматический доступ ограничен."
            );
        }

        if !status.is_success() {
            bail!("EDSM API вернул HTTP {status} для {final_url}");
        }

        if let Some(content_type) = content_type {
            if !content_type.contains("application/json") && !content_type.contains("+json") {
                bail!("Ожидался JSON от {final_url}, получен Content-Type: {content_type}");
            }
        }

        serde_json::from_str(&body)
            .with_context(|| format!("EDSM API вернул некорректный JSON для {final_url}"))
    }

    /// Загружает только успешную HTML-страницу EDSM. Оставлено для возможного
    /// резервного HTML-провайдера, хотя основной поиск использует JSON API.
    #[allow(dead_code)]
    ///
    /// Cloudflare challenge может возвращаться с HTTP 200, поэтому проверка
    /// статуса сама по себе недостаточна: проверяем также тело ответа.
    pub async fn fetch_html(&self, url: &str) -> Result<Html> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Ошибка HTTP-запроса к {url}"))?;

        let status = response.status();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_ascii_lowercase);

        let body = response
            .text()
            .await
            .with_context(|| format!("Не удалось прочитать ответ от {final_url}"))?;

        if looks_like_cloudflare_challenge(&body) {
            bail!(
                "EDSM вернул страницу проверки Cloudflare вместо данных ({final_url}, HTTP {status}). Автоматический доступ ограничен; повторите попытку позже."
            );
        }

        if !status.is_success() {
            bail!("EDSM вернул HTTP {status} для {final_url}");
        }

        if let Some(content_type) = content_type {
            if !content_type.contains("text/html")
                && !content_type.contains("application/xhtml+xml")
            {
                bail!(
                    "Ожидалась HTML-страница от {final_url}, получен Content-Type: {content_type}"
                );
            }
        }

        if body.trim().is_empty() {
            bail!("EDSM вернул пустой ответ для {final_url}");
        }

        Ok(Html::parse_document(&body))
    }
}

fn looks_like_cloudflare_challenge(body: &str) -> bool {
    let body = body.to_ascii_lowercase();

    [
        "cf-chl-",
        "challenge-platform",
        "cf-browser-verification",
        "__cf_chl_",
        "just a moment...",
        "attention required! | cloudflare",
        "performing security verification",
    ]
    .iter()
    .any(|marker| body.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::{looks_like_cloudflare_challenge, retry_delay};
    use reqwest::StatusCode;
    use std::time::Duration;

    #[test]
    fn detects_common_cloudflare_challenge_markers() {
        assert!(looks_like_cloudflare_challenge(
            "<html><title>Just a moment...</title><script src='/cdn-cgi/challenge-platform/x'></script></html>"
        ));
        assert!(looks_like_cloudflare_challenge(
            "<div id='cf-browser-verification'>Checking your browser</div>"
        ));
    }

    #[test]
    fn retries_transient_edsm_statuses_with_backoff() {
        assert_eq!(
            retry_delay(StatusCode::SERVICE_UNAVAILABLE, None, 0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            retry_delay(StatusCode::SERVICE_UNAVAILABLE, None, 2),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            retry_delay(StatusCode::TOO_MANY_REQUESTS, None, 0),
            Some(Duration::from_secs(2))
        );
    }

    #[test]
    fn respects_retry_after_seconds_and_ignores_permanent_errors() {
        assert_eq!(
            retry_delay(StatusCode::SERVICE_UNAVAILABLE, Some("7"), 0),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            retry_delay(StatusCode::BAD_REQUEST, Some("7"), 0),
            None
        );
    }

    #[test]
    fn accepts_regular_html_without_challenge_markers() {
        assert!(!looks_like_cloudflare_challenge(
            "<html><body><h1>EDSM</h1><table><tr><td>Planet</td></tr></table></body></html>"
        ));
    }
}
