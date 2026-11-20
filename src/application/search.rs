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
        Ok(SystemResolver::new(self.client.clone()).resolve(system_name).await?)
    }

    pub async fn execute(
        &self,
        center: &ResolvedSystem,
        criteria: &SearchCriteria,
    ) -> Result<Vec<Body>, SearchError> {
        Ok(BodySearch::new(self.client.clone()).search(center, criteria).await?)
    }
}
