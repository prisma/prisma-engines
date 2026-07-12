use quaint::prelude::{ExternalConnectionInfo, SqlFamily};
use serde::Deserialize;

// TODO: the code below largely duplicates driver_adapters::types, we should ideally use that
// crate instead, but it currently uses #cfg target a lot, which causes build issues when not
// explicitly building against wasm.

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsConnectionInfo {
    pub schema_name: Option<String>,
    pub max_bind_values: Option<u32>,
    pub supports_relation_joins: bool,
}

impl JsConnectionInfo {
    pub fn into_external_connection_info(self, provider: AdapterProvider) -> ExternalConnectionInfo {
        ExternalConnectionInfo::new(
            SqlFamily::from(provider),
            self.schema_name(provider).map(ToOwned::to_owned),
            self.max_bind_values.map(|v| v as usize),
            self.supports_relation_joins,
        )
    }

    fn schema_name(&self, provider: AdapterProvider) -> Option<&str> {
        self.schema_name
            .as_deref()
            .or_else(|| self.default_schema_name(provider))
    }

    fn default_schema_name(&self, provider: AdapterProvider) -> Option<&str> {
        match provider {
            #[cfg(feature = "mysql")]
            AdapterProvider::Mysql => None,
            #[cfg(feature = "postgresql")]
            AdapterProvider::Postgres => Some(quaint::connector::DEFAULT_POSTGRES_SCHEMA),
            #[cfg(feature = "sqlite")]
            AdapterProvider::Sqlite => Some(quaint::connector::DEFAULT_SQLITE_DATABASE),
            #[cfg(feature = "mssql")]
            AdapterProvider::SqlServer => Some(quaint::connector::DEFAULT_MSSQL_SCHEMA),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterProvider {
    #[cfg(feature = "mysql")]
    Mysql,
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "mssql")]
    #[serde(rename = "sqlserver")]
    SqlServer,
}

impl From<AdapterProvider> for SqlFamily {
    fn from(f: AdapterProvider) -> Self {
        match f {
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
