use quaint::prelude::{ExternalConnectionInfo, SqlFamily};
use serde::Deserialize;

#[cfg(feature = "kingbase-oracle")]
const DEFAULT_KINGBASE_ORACLE_SCHEMA: &str = "public";

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
        let info = ExternalConnectionInfo::new(
            SqlFamily::from(provider),
            self.schema_name(provider).map(ToOwned::to_owned),
            self.max_bind_values.map(|v| v as usize),
            self.supports_relation_joins,
        );

        #[cfg(feature = "kingbase-mysql")]
        if matches!(provider, AdapterProvider::KingbaseMysql) {
            return info.with_kingbase_mysql();
        }

        info
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
            #[cfg(feature = "kingbase-mysql")]
            AdapterProvider::KingbaseMysql => None,
            #[cfg(feature = "kingbase-oracle")]
            AdapterProvider::KingbaseOracle => Some(DEFAULT_KINGBASE_ORACLE_SCHEMA),
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
    #[cfg(feature = "kingbase-mysql")]
    #[serde(rename = "kingbase-mysql")]
    KingbaseMysql,
    #[cfg(feature = "kingbase-oracle")]
    #[serde(rename = "kingbase-oracle")]
    KingbaseOracle,
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
            #[cfg(feature = "kingbase-mysql")]
            AdapterProvider::KingbaseMysql => SqlFamily::Mysql,
            #[cfg(feature = "kingbase-oracle")]
            AdapterProvider::KingbaseOracle => SqlFamily::KingbaseOracle,
            #[cfg(feature = "postgresql")]
            AdapterProvider::Postgres => SqlFamily::Postgres,
            #[cfg(feature = "sqlite")]
            AdapterProvider::Sqlite => SqlFamily::Sqlite,
            #[cfg(feature = "mssql")]
            AdapterProvider::SqlServer => SqlFamily::Mssql,
        }
    }
}

#[cfg(all(test, feature = "kingbase-oracle"))]
mod tests {
    use super::{AdapterProvider, DEFAULT_KINGBASE_ORACLE_SCHEMA, JsConnectionInfo};
    use quaint::prelude::SqlFamily;

    #[test]
    fn kingbase_oracle_provider_uses_the_oracle_sql_family_and_default_schema() {
        let provider: AdapterProvider = serde_json::from_str("\"kingbase-oracle\"").unwrap();
        let connection_info = JsConnectionInfo::default().into_external_connection_info(provider);

        assert_eq!(connection_info.sql_family, SqlFamily::KingbaseOracle);
        assert_eq!(
            connection_info.schema_name.as_deref(),
            Some(DEFAULT_KINGBASE_ORACLE_SCHEMA)
        );
    }
}
