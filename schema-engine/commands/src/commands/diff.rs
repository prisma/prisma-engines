use std::sync::Arc;

use crate::{
    SchemaContainerExt,
    core_error::CoreResult,
    json_rpc::types::{DiffParams, DiffResult, DiffTarget, UrlContainer},
};
use enumflags2::BitFlags;
use json_rpc::types::MigrationList;
use schema_connector::{
    ConnectorError, ConnectorHost, DatabaseSchema, ExternalShadowDatabase, Namespaces, SchemaConnector, SchemaDialect,
};
use sql_schema_connector::SqlSchemaConnector;

// TODO: implement wasm32 version of this function.
pub async fn diff(params: DiffParams, host: Arc<dyn ConnectorHost>) -> CoreResult<DiffResult> {
    unimplemented!("diff");
}
