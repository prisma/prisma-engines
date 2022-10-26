use migration_core::{migration_connector::MigrationConnector, CoreResult};

#[must_use = "This struct does nothing on its own. See Reset::send()"]
pub struct Reset<'a> {
    api: &'a mut dyn MigrationConnector,
    soft: bool,
}

impl<'a> Reset<'a> {
    pub fn new(api: &'a mut dyn MigrationConnector) -> Self {
        Reset { api, soft: false }
    }

    pub fn soft(mut self, value: bool) -> Self {
        self.soft = value;
        self
    }

    pub async fn send(self) -> CoreResult<ResetAssertion> {
        // TODO: should we somehow send Namespaces here?
        self.api.reset(self.soft, None).await?;

        Ok(ResetAssertion {})
    }

    /// Execute the command and expect it to succeed.
    #[track_caller]
    pub fn send_sync(self) -> ResetAssertion {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap()
    }
}

pub struct ResetAssertion {}

impl std::fmt::Debug for ResetAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResetAssertion {{ .. }}")
    }
}
