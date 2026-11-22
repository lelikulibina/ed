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
