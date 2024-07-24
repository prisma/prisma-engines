use schema_core::{
    schema_connector::{IntrospectSqlQueryInput, IntrospectSqlQueryOutput, SchemaConnector},
    CoreError, CoreResult,
};

#[must_use = "This struct does nothing on its own. See ApplyMigrations::send()"]
pub struct IntrospectSql<'a> {
    api: &'a mut dyn SchemaConnector,
    name: &'a str,
    source: String,
}

impl<'a> IntrospectSql<'a> {
    pub fn new(api: &'a mut dyn SchemaConnector, name: &'a str, source: String) -> Self {
        Self {
            api,
            name: name,
            source,
        }
    }

    pub async fn send(self) -> CoreResult<IntrospectSqlAssertion> {
        let res = self
            .api
            .introspect_sql(IntrospectSqlQueryInput {
                name: self.name.to_owned(),
                source: self.source,
            })
            .await?;

        Ok(IntrospectSqlAssertion { output: res })
    }

    #[track_caller]
    pub fn send_sync(self) -> IntrospectSqlAssertion {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap_err()
    }
}

pub struct IntrospectSqlAssertion {
    output: IntrospectSqlQueryOutput,
}

impl std::fmt::Debug for IntrospectSqlAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyMigrationsAssertion {{ .. }}")
    }
}

impl IntrospectSqlAssertion {
    #[track_caller]
    pub fn expect_result(self, expectation: expect_test::Expect) {
        expectation.assert_debug_eq(&self.output)
    }
}
