use super::*;
use crate::{BoxFuture, SqlDatamodelRenderer};
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default)]
pub struct SqliteConnectorTag;

impl ConnectorTagInterface for SqliteConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "sqlite"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::SQLITE.capabilities()
    }
}
