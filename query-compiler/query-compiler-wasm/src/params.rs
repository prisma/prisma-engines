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
}

impl JsConnectionInfo {
    pub fn into_external_connection_info(self, provider: AdapterFlavour) -> ExternalConnectionInfo {
        let schema_name = self.get_schema_name(provider);
        let sql_family = SqlFamily::from(provider);

        ExternalConnectionInfo::new(
            sql_family,
            schema_name.to_owned(),
            self.max_bind_values.map(|v| v as usize),
        )
    }

    fn get_schema_name(&self, provider: AdapterFlavour) -> &str {
        match self.schema_name.as_ref() {
            Some(name) => name,
            None => self.default_schema_name(provider),
        }
    }

    fn default_schema_name(&self, provider: AdapterFlavour) -> &str {
        match provider {
            #[cfg(feature = "mysql")]
            AdapterFlavour::Mysql => quaint::connector::DEFAULT_MYSQL_DB,
            #[cfg(feature = "postgresql")]
            AdapterFlavour::Postgres => quaint::connector::DEFAULT_POSTGRES_SCHEMA,
            #[cfg(feature = "sqlite")]
            AdapterFlavour::Sqlite => quaint::connector::DEFAULT_SQLITE_DATABASE,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterFlavour {
    #[cfg(feature = "mysql")]
    Mysql,
    #[cfg(feature = "postgresql")]
    Postgres,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

impl From<AdapterFlavour> for SqlFamily {
    fn from(f: AdapterFlavour) -> Self {
        match f {
            #[cfg(feature = "mysql")]
            AdapterFlavour::Mysql => SqlFamily::Mysql,
            #[cfg(feature = "postgresql")]
            AdapterFlavour::Postgres => SqlFamily::Postgres,
            #[cfg(feature = "sqlite")]
            AdapterFlavour::Sqlite => SqlFamily::Sqlite,
        }
    }
}
