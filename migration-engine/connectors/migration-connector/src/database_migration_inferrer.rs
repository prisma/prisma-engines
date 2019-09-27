use crate::{ConnectorResult, MigrationStep};
use datamodel::Datamodel;

/// The component responsible for generating a [DatabaseMigration](trait.MigrationConnector.html#associatedtype.DatabaseMigration)
/// migrating the database from one datamodel to another. In addition to the datamodel information provided by the core,
/// the component has access to the database, e.g. for introspection.
pub trait DatabaseMigrationInferrer<T>: Send + Sync + 'static {
    fn infer(&self, previous: &Datamodel, next: &Datamodel, steps: &Vec<MigrationStep>) -> ConnectorResult<T>;
}
