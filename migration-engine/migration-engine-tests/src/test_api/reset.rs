use migration_core::{CoreResult, GenericApi};

#[must_use = "This struct does nothing on its own. See Reset::send()"]
pub struct Reset<'a> {
    api: &'a dyn GenericApi,
    rt: Option<&'a tokio::runtime::Runtime>,
}

impl<'a> Reset<'a> {
    pub fn new(api: &'a dyn GenericApi) -> Self {
        Reset { api, rt: None }
    }

    pub fn new_sync(api: &'a dyn GenericApi, rt: &'a tokio::runtime::Runtime) -> Self {
        Reset { api, rt: Some(rt) }
    }

    pub async fn send(self) -> CoreResult<ResetAssertion<'a>> {
        self.api.reset().await?;

        Ok(ResetAssertion { _api: self.api })
    }

    /// Execute the command and expect it to succeed.
    #[track_caller]
    pub fn send_sync(self) -> ResetAssertion<'a> {
        self.rt.unwrap().block_on(self.send()).unwrap()
    }
}

pub struct ResetAssertion<'a> {
    _api: &'a dyn GenericApi,
}

impl std::fmt::Debug for ResetAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResetAssertion {{ .. }}")
    }
}
