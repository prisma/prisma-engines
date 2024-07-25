use quaint::prelude::ColumnType;
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
        Self { api, name, source }
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
    pub output: IntrospectSqlQueryOutput,
}

impl std::fmt::Debug for IntrospectSqlAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyMigrationsAssertion {{ .. }}")
    }
}

impl IntrospectSqlAssertion {
    #[track_caller]
    pub fn expect_result(&self, expectation: expect_test::Expect) {
        expectation.assert_debug_eq(&self.output)
    }

    #[track_caller]
    pub fn expect_param_type(self, idx: usize, ty: ColumnType) -> Self {
        let param = &self
            .output
            .parameters
            .get(idx)
            .unwrap_or_else(|| panic!("parameter at index {idx} not found"));
        let param_name = &param.name;
        let actual_typ = &param.typ;
        let expected_typ = &ty.to_string();

        assert_eq!(
            expected_typ, actual_typ,
            "expected param {param_name} to be of type {expected_typ}, got: {actual_typ}",
        );

        self
    }

    #[track_caller]
    pub fn expect_column_type(self, idx: usize, ty: ColumnType) -> Self {
        let column = &self
            .output
            .result_columns
            .get(idx)
            .unwrap_or_else(|| panic!("column at index {idx} not found"));
        let column_name = &column.name;
        let actual_typ = &column.typ;
        let expected_typ = &ty.to_string();

        assert_eq!(
            expected_typ, actual_typ,
            "expected column {column_name} to be of type {expected_typ}, got: {actual_typ}"
        );

        self
    }
}
