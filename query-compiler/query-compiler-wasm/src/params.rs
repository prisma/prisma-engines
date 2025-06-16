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
        let schema_name = self.get_schema_name(provider);
        let sql_family = SqlFamily::from(provider);

        wasm_rs_dbg::dbg!(format!(
            "into_external_connection_info: {:?}; {}",
            &sql_family, self.supports_relation_joins
        ));
        // ==> into_external_connection_info: Mysql; true

        ExternalConnectionInfo::new(
            sql_family,
            schema_name.to_owned(),
            self.max_bind_values.map(|v| v as usize),
            self.supports_relation_joins,
        )
    }

    fn get_schema_name(&self, provider: AdapterProvider) -> &str {
        match self.schema_name.as_ref() {
            Some(name) => name,
            None => self.default_schema_name(provider),
        }
    }

    fn default_schema_name(&self, provider: AdapterProvider) -> &str {
        match provider {
            #[cfg(feature = "mysql")]
            AdapterProvider::Mysql => quaint::connector::DEFAULT_MYSQL_DB,
            #[cfg(feature = "postgresql")]
            AdapterProvider::Postgres => quaint::connector::DEFAULT_POSTGRES_SCHEMA,
            #[cfg(feature = "sqlite")]
            AdapterProvider::Sqlite => quaint::connector::DEFAULT_SQLITE_DATABASE,
            #[cfg(feature = "mssql")]
            AdapterProvider::SqlServer => quaint::connector::DEFAULT_MSSQL_SCHEMA,
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
