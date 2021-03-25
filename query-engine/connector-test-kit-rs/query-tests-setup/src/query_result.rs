use request_handlers::PrismaResponse;

pub struct QueryResult {
    response: PrismaResponse,
}

impl QueryResult {
    pub fn failed(&self) -> bool {
        match self.response {
            PrismaResponse::Single(ref s) => s.errors().next().is_some(),
            PrismaResponse::Multi(ref m) => m.errors().next().is_some(),
        }
    }

    /// Asserts absence of errors in the result. Panics with assertion error.
    pub fn assert_success(&self) {
        assert!(!self.failed())
    }

    pub fn assert_failure(&self, _err_code: usize, _msg_contains: Option<String>) {
        todo!()
    }
}

impl ToString for QueryResult {
    fn to_string(&self) -> String {
        serde_json::to_value(&self.response).unwrap().to_string()
    }
}

impl From<PrismaResponse> for QueryResult {
    fn from(response: PrismaResponse) -> Self {
        Self { response }
    }
}
