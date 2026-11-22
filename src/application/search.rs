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
    use crate::domain::{
        catalog::{AtmosphereType, SurfaceType},
        criteria::{Filter, NumericRange},
    };
    use crate::spansh::test_support::{MockResponse, MockServer};

    #[tokio::test]
    async fn resolves_center_searches_bodies_and_returns_filtered_results() {
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
        let server = MockServer::start(responses.into_iter().map(MockResponse::ok).collect()).await;
        let client = SpanshClient::new(server.base_url(), 1).unwrap();
        let search = SearchUseCase::new(client);
        let center = search.resolve_center("Sol").await.unwrap();
        let criteria = SearchCriteria::new(
            Filter::one_of(vec![SurfaceType::RockyBody]),
            Filter::one_of(vec![AtmosphereType::Nitrogen]),
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

        let requests = server.finish().await;
        assert!(requests[0].starts_with("GET /api/systems/field_values/system_names?q=Sol "));
        assert!(requests[1].starts_with("GET /api/system/42 "));
        assert!(requests[2].starts_with("POST /api/bodies/search "));
    }
}
