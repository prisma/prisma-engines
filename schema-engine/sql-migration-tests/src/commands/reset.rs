use schema_core::{
    schema_connector::{Namespaces, SchemaConnector, SchemaFilter},
    CoreResult,
};

#[must_use = "This struct does nothing on its own. See Reset::send()"]
pub struct Reset<'a> {
    api: &'a mut dyn SchemaConnector,
    soft: bool,
    filter: SchemaFilter,
}

impl<'a> Reset<'a> {
    pub fn new(api: &'a mut dyn SchemaConnector) -> Self {
        Reset {
            api,
            soft: false,
            filter: SchemaFilter::default(),
        }
    }

    pub fn soft(mut self, value: bool) -> Self {
        self.soft = value;
        self
    }

    pub fn filter(mut self, filter: SchemaFilter) -> Self {
        self.filter = filter;
        self
    }

    pub async fn send(self, namespaces: Option<Namespaces>) -> CoreResult<ResetAssertion> {
        self.api.reset(self.soft, namespaces, &self.filter).await?;

        Ok(ResetAssertion {})
    }

    /// Execute the command and expect it to succeed.
    #[track_caller]
    pub fn send_sync(self, namespaces: Option<Namespaces>) -> ResetAssertion {
        test_setup::runtime::run_with_thread_local_runtime(self.send(namespaces)).unwrap()
    }
}

pub struct ResetAssertion {}

impl std::fmt::Debug for ResetAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResetAssertion {{ .. }}")
    }
}
