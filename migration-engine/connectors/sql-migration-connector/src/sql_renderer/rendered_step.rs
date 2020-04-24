pub(crate) struct RenderedStep {
    statements: Vec<String>,
    run_in_transaction: bool,
}

impl RenderedStep {
    pub(crate) fn new(statements: Vec<String>) -> Self {
        RenderedStep {
            statements,
            run_in_transaction: false,
        }
    }

    pub(crate) fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub(crate) fn with_transaction(mut self, run_in_transaction: bool) -> Self {
        self.run_in_transaction = run_in_transaction;

        self
    }
}

// TEMPORARY
impl Into<Result<Vec<String>, anyhow::Error>> for RenderedStep {
    fn into(self) -> Result<Vec<String>, anyhow::Error> {
        Ok(self.statements)
    }
}
