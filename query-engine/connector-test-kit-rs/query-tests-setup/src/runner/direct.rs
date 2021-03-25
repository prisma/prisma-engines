use std::sync::Arc;

use crate::{RunnerInterface, TestResult};
use prisma_models::DatamodelConverter;
use query_core::{exec_loader, schema_builder, BuildMode, QueryExecutor, QuerySchemaRef};
use request_handlers::{GraphQlBody, GraphQlHandler, MultiQuery};

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct DirectRunner {
    executor: Executor,
    query_schema: QuerySchemaRef,
}

#[async_trait::async_trait]
impl RunnerInterface for DirectRunner {
    async fn load(datamodel: String) -> TestResult<Self> {
        feature_flags::initialize(&["all".to_owned()]).unwrap();

        dbg!(feature_flags::get());

        let config = datamodel::parse_configuration_with_url_overrides(&datamodel, vec![])
            .unwrap()
            .subject;

        let parsed_datamodel = datamodel::parse_datamodel(&datamodel).unwrap().subject;
        let internal_datamodel = DatamodelConverter::convert(&parsed_datamodel);
        let data_source = config.datasources.first().expect("No valid data source found");
        let (db_name, executor) = exec_loader::load(&data_source).await?;
        let internal_data_model = internal_datamodel.build(db_name);

        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            BuildMode::Modern,
            true,
            data_source.capabilities(),
        ));

        Ok(Self { executor, query_schema })
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
}
