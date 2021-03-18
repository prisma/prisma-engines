use request_handlers::PrismaResponse;

pub struct QueryResult {
    response: PrismaResponse,
}

impl QueryResult {
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
