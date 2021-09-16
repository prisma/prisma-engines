use request_handlers::{GQLError, PrismaResponse};

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

    /// Asserts presence of errors in the result.
    /// Code must equal the given one, the message is a partial match.
    /// If more than one error is contained, asserts that at least one error contains the message _and_ code.
    ///
    /// Panics with assertion error on no match.
    pub fn assert_failure(&self, err_code: usize, msg_contains: Option<String>) {
        if !self.failed() {
            panic!(
                "Expected result to return an error, but found success: {}",
                self.to_string()
            );
        }

        // 0 is the "do nothing marker"
        if err_code == 0 {
            return;
        }

        let err_code = format!("P{}", err_code);
        let err_exists = self.errors().into_iter().any(|err| {
            let code_matches = err.code() == Some(&err_code);
            let msg_matches = match msg_contains.as_ref() {
                Some(msg) => err.message().contains(msg),
                None => true,
            };

            code_matches && msg_matches
        });

        if !err_exists {
            if let Some(msg) = msg_contains {
                panic!(
                    "Expected error with code `{}` and message `{}`, got: `{}`",
                    err_code,
                    msg,
                    self.to_string()
                );
            } else {
                panic!("Expected error with code `{}`, got: `{}`", err_code, self.to_string());
            }
        }
    }

    pub fn errors(&self) -> Vec<&GQLError> {
        match self.response {
            PrismaResponse::Single(ref s) => s.errors().collect(),
            PrismaResponse::Multi(ref m) => m.errors().collect(),
        }
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
