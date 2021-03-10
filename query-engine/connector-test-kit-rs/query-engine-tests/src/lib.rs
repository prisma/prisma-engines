use query_test_macros::connector_test;
use serde_json::Value;

pub type TestResult = anyhow::Result<()>;

// todo
pub struct QueryResult {
    json: Value,
}

impl QueryResult {
    pub fn assert_failure(&self, code: usize, msg_contains: Option<String>) {
        todo!()
    }
}

impl ToString for QueryResult {
    fn to_string(&self) -> String {
        todo!()
    }
}

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NApi(NApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

impl Runner {
    pub fn load() -> Self {
        todo!()
    }

    pub fn query<T>(&self, gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }

    pub fn batch<T>(&self, gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }
}

pub struct DirectRunner {}
pub struct NApiRunner {}
pub struct BinaryRunner {}

pub enum ConnectorTag {
    SqlServer(Option<SqlServerVersion>),
    MySql(Option<MySqlVersion>),
    Postgres(Option<PostgresVersion>),
    Sqlite,
    MongoDb,
}

/// WIP, looking at ideas how to have a good tag API (Q: usable in the macros?).
impl ConnectorTag {
    pub fn postgres() -> Self {
        Self::Postgres(None)
    }

    pub fn postgres_9() -> Self {
        Self::Postgres(Some(PostgresVersion::V9))
    }
}

pub enum SqlServerVersion {
    V_2017,
    V_2019,
}

pub enum MySqlVersion {
    V5_6,
    V5_7,
    V8,
}

pub enum PostgresVersion {
    V9,
    V10,
    V11,
    V12,
}

/// Wip, just a collection of env vars we might want.
struct EnvConfig {
    /// MIGRATION_ENGINE_PATH
    migration_engine_path: String,

    /// TEST_RUNNER
    runner: String,
}

// The mod name dictates the db name. If the name is `some_spec`
// then, for example, the MySQL db should be (similar to) `some_spec` as well.
#[cfg(test)]
// #[before_each(before_each_handler)] // Hook to run before each test.
// #[schema(schema_handler)] // Schema for all contained tests. Allows us to cache runners maybe.
mod some_spec {
    use super::*;

    // fn before_each_handler(runner: &Runner) {
    //     // Maybe we don't need this.
    //     runner.truncate_data(); // Actually, this should always happen for a connector test.
    //     test_data(); // This can also be done in each test manually or by convention.
    // }

    fn test_data() {
        todo!()
    }

    // Handler that returns a schema template to use for rendering.
    // Template rendering can be bypassed by simply not using the template strings.
    // Common schema handlers to use should be in a central place.
    fn schema_handler() -> String {
        "model A {
            #id(id, Int, @id)
            field String?
            #relation(bs, B, ...)
        }"
        .to_owned()
    }

    #[connector_test(
        schema(schemahandler), // Override or manual set of schema to use.
        only(Postgres), // Only run for certain connectors, xor with `exclude`
        exclude(SqlServer) // Run for all except certain connectors, xor with `only`
        // If none of the two above are specified all connectors are run.
    )]
    fn ideal_api_test(runner: &Runner) {
        let result = runner.query(
            "
            mutation {
                createOneA(data: {...}) { id }
            }
        ",
        );

        assert_eq!(result.to_string(), r#"{"data":{"createOneA":[...]}}"#);
    }
}
