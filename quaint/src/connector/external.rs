use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;

use super::{SqlFamily, TransactionCapable};

#[cfg_attr(target_arch = "wasm32", derive(serde::Deserialize))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum AdapterD1 {
    Env,
    HTTP,
}

#[cfg_attr(target_arch = "wasm32", derive(serde::Deserialize))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
/// The name of the adapter.
/// We only want to keep track of first-class adapters maintained by Prisma, and among those,
/// only the ones whose queries require special handling compared to the ones generated via `quaint`.
///
/// TODO: we could add here Neon as well, so we could exclude / expose Neon's auth tables in the future.
pub enum AdapterName {
    D1(AdapterD1),
    LibSQL,
    BetterSQLite3,
    Planetscale,
    Mssql,
    Unknown,
}

impl FromStr for AdapterName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip `@prisma/adapter-` prefix from the string
        if let Some(name) = s.strip_prefix("@prisma/adapter-") {
            match name {
                "d1" => Ok(Self::D1(AdapterD1::Env)),
                "d1-http" => Ok(Self::D1(AdapterD1::HTTP)),
                "libsql" => Ok(Self::LibSQL),
                "better-sqlite3" => Ok(Self::BetterSQLite3),
                "planetscale" => Ok(Self::Planetscale),
                "mssql" => Ok(Self::Mssql),
                _ => Ok(Self::Unknown),
            }
        } else {
            Ok(Self::Unknown)
        }
    }
}

#[cfg_attr(target_arch = "wasm32", derive(serde::Deserialize))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum AdapterProvider {
    #[cfg(feature = "mysql")]
    Mysql,
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "mssql")]
    #[cfg_attr(target_arch = "wasm32", serde(rename = "sqlserver"))]
    SqlServer,
}

impl AdapterProvider {
    pub fn db_system_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "mysql")]
            Self::Mysql => "mysql",
            #[cfg(feature = "postgresql")]
            Self::Postgres => "postgresql",
            #[cfg(feature = "sqlite")]
            Self::Sqlite => "sqlite",
            #[cfg(feature = "mssql")]
            Self::SqlServer => "mssql",
        }
    }
}

impl FromStr for AdapterProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "postgresql")]
            "postgres" => Ok(Self::Postgres),
            #[cfg(feature = "mysql")]
            "mysql" => Ok(Self::Mysql),
            #[cfg(feature = "sqlite")]
            "sqlite" => Ok(Self::Sqlite),
            #[cfg(feature = "mssql")]
            "sqlserver" => Ok(Self::SqlServer),
            _ => Err(format!("Unsupported adapter flavour: {s:?}")),
        }
    }
}

impl From<&AdapterProvider> for SqlFamily {
    fn from(value: &AdapterProvider) -> Self {
        match value {
            #[cfg(feature = "mysql")]
            AdapterProvider::Mysql => SqlFamily::Mysql,
            #[cfg(feature = "postgresql")]
            AdapterProvider::Postgres => SqlFamily::Postgres,
            #[cfg(feature = "sqlite")]
            AdapterProvider::Sqlite => SqlFamily::Sqlite,
            #[cfg(feature = "mssql")]
            AdapterProvider::SqlServer => SqlFamily::Mssql,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalConnectionInfo {
    // TODO: `sql_family` doesn't exist in TypeScript's `ConnectionInfo` type.
    pub sql_family: SqlFamily,
    pub schema_name: Option<String>,
    pub max_bind_values: Option<usize>,
    pub supports_relation_joins: bool,
}

impl ExternalConnectionInfo {
    pub fn new(
        sql_family: SqlFamily,
        schema_name: Option<String>,
        max_bind_values: Option<usize>,
        supports_relation_joins: bool,
    ) -> Self {
        ExternalConnectionInfo {
            sql_family,
            schema_name,
            max_bind_values,
            supports_relation_joins,
        }
    }
}

#[async_trait]
pub trait ExternalConnector: TransactionCapable {
    fn adapter_name(&self) -> AdapterName;
    fn provider(&self) -> AdapterProvider;
    async fn get_connection_info(&self) -> crate::Result<ExternalConnectionInfo>;
    async fn execute_script(&self, script: &str) -> crate::Result<()>;
    async fn dispose(&self) -> crate::Result<()>;

    /// Returns a reference to self as an ExternalConnector.
    fn as_external_connector(&self) -> Option<&dyn ExternalConnector>
    where
        Self: Sized,
    {
        Some(self)
    }
}

#[async_trait]
pub trait ExternalConnectorFactory: Send + Sync {
    fn adapter_name(&self) -> AdapterName;
    fn provider(&self) -> AdapterProvider;
    async fn connect(&self) -> crate::Result<Arc<dyn ExternalConnector>>;
    async fn connect_to_shadow_db(&self) -> Option<crate::Result<Arc<dyn ExternalConnector>>>;
}
