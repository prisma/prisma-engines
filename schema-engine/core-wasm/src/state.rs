use crate::api::GenericApi;
use quaint::connector::ExternalConnector;
use std::sync::Arc;

// Blanket impl for any Arc<T> where T: ExternalConnector + ?Sized
#[async_trait::async_trait]
impl<T> GenericApi for Arc<T>
where
    T: ExternalConnector + ?Sized,
{
    async fn version(&self, _params: Option<GetDatabaseVersionInput>) -> CoreResult<String> {
        Ok("1.0.0".to_string())
    }

    async fn ensure_connection_validity(
        &self,
        _params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult> {
        Ok(EnsureConnectionValidityResult {})
    }
}

pub struct EngineState {
    adapter: Arc<dyn ExternalConnector>,
}

impl GenericApi {
    pub fn new(adapter: Arc<dyn ExternalConnector>) -> Self {
        Self { adapter }
    }
}

#[async_trait::async_trait]
impl GenericApi for EngineState {
    async fn version(&self, _params: Option<GetDatabaseVersionInput>) -> CoreResult<String> {
        Ok("1.0.0".to_string())
    }

    async fn ensure_connection_validity(
        &self,
        _params: EnsureConnectionValidityParams,
    ) -> CoreResult<EnsureConnectionValidityResult> {
        Ok(EnsureConnectionValidityResult {})
    }
}
