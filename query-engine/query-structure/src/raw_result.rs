use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct RawResult(Arc<dyn erased_serde::Serialize + Send + Sync>);

impl RawResult {
    pub fn new<T>(data: T) -> Self
    where
        T: Serialize + Send + Sync + 'static,
    {
        Self(Arc::new(data))
    }
}

impl Serialize for RawResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl std::fmt::Debug for RawResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RawResult").field(&"<dyn Serialize>").finish()
    }
}
