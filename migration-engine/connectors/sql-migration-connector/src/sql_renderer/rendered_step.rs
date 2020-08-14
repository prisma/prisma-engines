pub(crate) struct RenderedStep {
    statements: Vec<String>,
}

impl RenderedStep {
    pub(crate) fn new(statements: Vec<String>) -> Self {
        RenderedStep { statements }
    }
}

// TEMPORARY
impl Into<Result<Vec<String>, anyhow::Error>> for RenderedStep {
    fn into(self) -> Result<Vec<String>, anyhow::Error> {
        Ok(self.statements)
    }
}
