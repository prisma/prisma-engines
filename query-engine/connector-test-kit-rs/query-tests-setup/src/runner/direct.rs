use prisma_models::DatamodelConverter;
use query_core::{QueryExecutor, QuerySchemaRef};

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct DirectRunner {
    executor: Executor,
    query_schema: QuerySchemaRef,
}

impl DirectRunner {
    pub fn load(datamodel: &str) -> Self {
        let template = DatamodelConverter::convert(&dm);

        // We only support one data source at the moment, so take the first one (default not exposed yet).
        let data_source = config.datasources.first().expect("No valid data source found");

        // Load executor
        let (db_name, executor) = exec_loader::load(&data_source).await?;

        // Build internal data model
        let internal_data_model = template.build(db_name);

        // Construct query schema
        let build_mode = if legacy { BuildMode::Legacy } else { BuildMode::Modern };
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            build_mode,
            enable_raw_queries,
            data_source.capabilities(),
        ));

        todo!()
    }
}

// let handler = GraphQlHandler::new(engine.executor(), engine.query_schema());
//                         Ok(handler.handle(query).await)
