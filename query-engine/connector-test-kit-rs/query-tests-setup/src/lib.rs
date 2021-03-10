use serde_json::Value;

// todo
pub struct QueryResult {
    json: Value,
}

impl QueryResult {
    pub fn assert_failure(&self, err_code: usize, msg_contains: Option<String>) {
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
        println!("Totally loaded");
        Self::Direct(DirectRunner {})
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
