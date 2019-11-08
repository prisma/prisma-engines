pub mod calculate_datamodel; // only exported to be able to unit test it

mod error;
mod schema_describer_loading;

use datamodel::Datamodel;
use introspection_connector::{ConnectorResult, DatabaseMetadata, IntrospectionConnector};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};

pub use error::*;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlIntrospectionError>;

pub struct SqlIntrospectionConnector {
    describer: Box<dyn SqlSchemaDescriberBackend>,
}

impl SqlIntrospectionConnector {
    pub fn new(url: &str) -> ConnectorResult<SqlIntrospectionConnector> {
        let describer = schema_describer_loading::load_describer(&url)?;
        Ok(SqlIntrospectionConnector { describer })
    }

    fn list_databases_internal(&self) -> SqlIntrospectionResult<Vec<String>> {
        Ok(self.describer.list_databases()?)
    }

    fn get_metadata_internal(&self, database: &str) -> SqlIntrospectionResult<DatabaseMetadata> {
        let sql_metadata = self.describer.get_metadata(&database)?;
        let db_metadate = DatabaseMetadata {
            table_count: sql_metadata.table_count,
            size_in_bytes: sql_metadata.size_in_bytes,
        };
        Ok(db_metadate)
    }

    fn describe(&self, database: &str) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self.describer.describe(&database)?)
    }
}

impl IntrospectionConnector for SqlIntrospectionConnector {
    fn list_databases(&self) -> ConnectorResult<Vec<String>> {
        Ok(self.list_databases_internal()?)
    }

    fn get_metadata(&self, database: &str) -> ConnectorResult<DatabaseMetadata> {
        Ok(self.get_metadata_internal(&database)?)
    }

    fn introspect(&self, database: &str) -> ConnectorResult<Datamodel> {
        let sql_schema = self.describe(database)?;
        let data_model = calculate_datamodel::calculate_model(&sql_schema).unwrap();
        Ok(data_model)
    }
}
