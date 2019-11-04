use datamodel::ast::SchemaAst;
use migration_connector::steps::MigrationStep;

pub trait DataModelMigrationStepsInferrer: Send + Sync + 'static {
    fn infer(&self, previous: &SchemaAst, next: &SchemaAst) -> Vec<MigrationStep>;
}

pub struct DataModelMigrationStepsInferrerImplWrapper {}

impl DataModelMigrationStepsInferrer for DataModelMigrationStepsInferrerImplWrapper {
    fn infer(&self, previous: &SchemaAst, next: &SchemaAst) -> Vec<MigrationStep> {
        let inferrer = DataModelMigrationStepsInferrerImpl { previous, next };
        crate::migration::datamodel_differ::diff(inferrer.previous, inferrer.next)
    }
}

#[allow(dead_code)]
pub struct DataModelMigrationStepsInferrerImpl<'a> {
    previous: &'a SchemaAst,
    next: &'a SchemaAst,
}
