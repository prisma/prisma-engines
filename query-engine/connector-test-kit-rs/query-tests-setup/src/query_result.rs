use request_handlers::{GQLError, PrismaResponse};

#[derive(Debug)]
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
        if self.failed() {
            panic!("{}", self.to_string());
        }
    }

    /// Asserts presence of errors in the result.
    /// Code must equal the given one, the message is a partial match.
    /// If more than one error is contained, asserts that at least one error contains the message _and_ code.
    ///
    /// Panics with assertion error on no match.
    pub fn assert_failure(&self, err_code: impl Into<Option<usize>>, msg_contains: Option<String>) {
        let err_code: Option<usize> = err_code.into();
        if !self.failed() {
            panic!(
                "Expected result to return an error, but found success: {}",
                self.to_string()
            );
        }

        // 0 is the "do nothing marker"
        if err_code == Some(0) {
            return;
        }

        let err_code = err_code.map(|code| format!("P{code}"));
        let err_exists = self.errors().into_iter().any(|err| {
            let code_matches = err.code() == err_code.as_deref();
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
                    err_code.unwrap_or_else(|| "None".to_owned()),
                    msg,
                    self.to_string()
                );
            } else {
                panic!(
                    "Expected error with code `{}`, got: `{}`",
                    err_code.unwrap_or_else(|| "None".to_owned()),
                    self.to_string()
                );
            }
        }
    }

    pub fn errors(&self) -> Vec<&GQLError> {
        match self.response {
            PrismaResponse::Single(ref s) => s.errors().collect(),
            PrismaResponse::Multi(ref m) => m.errors().collect(),
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(&self.response).unwrap()
    }

    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.response).unwrap()
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
