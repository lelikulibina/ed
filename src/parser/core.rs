use anyhow::Result;
use reqwest::Client;
use scraper::Html;

/// Основной парсер с HTTP-клиентом
#[derive(Clone)]
pub struct ParserCore {
    pub client: Client,
}

impl ParserCore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                             AppleWebKit/537.36 (KHTML, like Gecko) \
                             Chrome/142.0.0.0 Safari/537.36")
                .build()?,
        })
    }

    pub async fn fetch_html(&self, url: &str) -> Result<Html> {
        let text = self.client.get(url).send().await?.text().await?;
        Ok(Html::parse_document(&text))
    }
}
