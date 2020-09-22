use migration_core::GenericApi;

#[must_use = "This struct does nothing on its own. See Reset::send()"]
pub struct Reset<'a> {
    api: &'a dyn GenericApi,
}

impl<'a> Reset<'a> {
    pub fn new(api: &'a dyn GenericApi) -> Self {
        Reset { api }
    }

    pub async fn send(self) -> anyhow::Result<ResetAssertion<'a>> {
        self.api.reset(&()).await?;

        Ok(ResetAssertion { _api: self.api })
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
