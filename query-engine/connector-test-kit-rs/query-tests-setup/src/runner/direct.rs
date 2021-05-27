use std::sync::Arc;

use crate::{ConnectorTag, RunnerInterface, TestResult};
use prisma_models::DatamodelConverter;
use query_core::{exec_loader, schema_builder, BuildMode, QueryExecutor, QuerySchemaRef};
use request_handlers::{GraphQlBody, GraphQlHandler, MultiQuery};

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct DirectRunner {
    executor: Executor,
    query_schema: QuerySchemaRef,
    connector_tag: ConnectorTag,
}

#[async_trait::async_trait]
impl RunnerInterface for DirectRunner {
    async fn load(datamodel: String, connector_tag: ConnectorTag) -> TestResult<Self> {
        let config = datamodel::parse_configuration(&datamodel).unwrap().subject;

        let parsed_datamodel = datamodel::parse_datamodel(&datamodel).unwrap().subject;
        let internal_datamodel = DatamodelConverter::convert(&parsed_datamodel);
        let data_source = config.datasources.first().expect("No valid data source found");
        let preview_features: Vec<_> = config.preview_features().cloned().collect();
        let url = data_source.load_url().unwrap();
        let (db_name, executor) = exec_loader::load(&data_source, &preview_features, &url).await?;
        let internal_data_model = internal_datamodel.build(db_name);

        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            BuildMode::Modern,
            true,
            data_source.capabilities(),
            preview_features,
        ));

        Ok(Self {
            executor,
            query_schema,
            connector_tag,
        })
    }

    async fn query(&self, query: String) -> TestResult<crate::QueryResult> {
        let handler = GraphQlHandler::new(&*self.executor, &self.query_schema);
        let query = GraphQlBody::Single(query.into());

        Ok(handler.handle(query).await.into())
    }

    async fn batch(&self, queries: Vec<String>, transaction: bool) -> TestResult<crate::QueryResult> {
        let handler = GraphQlHandler::new(&*self.executor, &self.query_schema);
        let query = GraphQlBody::Multi(MultiQuery::new(
            queries.into_iter().map(Into::into).collect(),
            transaction,
        ));

        Ok(handler.handle(query).await.into())
    }

    fn connector(&self) -> &crate::ConnectorTag {
        &self.connector_tag
    }
}
