use crate::{Filter, QueryArguments, RecordFinder, WriteArgs};
use prisma_models::*;

pub trait Connector {
    fn get_connection<'a>(&'a self) -> crate::IO<Box<dyn Connection + 'a>>;
}

pub trait Connection: ReadOperations + WriteOperations + Send + Sync {
    fn start_transaction<'a>(&'a self) -> crate::IO<Box<dyn Transaction + 'a>>;
}

pub trait Transaction<'a>: ReadOperations + WriteOperations + Send + Sync {
    fn commit<'b>(&'b self) -> crate::IO<'b, ()>;
    fn rollback<'b>(&'b self) -> crate::IO<'b, ()>;
}

pub trait ReadOperations {
    fn get_single_record<'a>(
        &'a self,
        record_finder: &'a RecordFinder,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, Option<SingleRecord>>;

    fn get_many_records(
        &self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> crate::IO<ManyRecords>;

    fn get_related_records(
        &self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> crate::IO<ManyRecords>;

    // This method is temporary
    fn get_scalar_list_values(
        &self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> crate::IO<Vec<ScalarListValues>>;

    // This will eventually become a more generic `aggregate`
    fn count_by_model(&self, model: ModelRef, query_arguments: QueryArguments) -> crate::IO<usize>;
}

#[derive(Debug, Clone)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}

pub trait WriteOperations {
    fn create_record(&self, model: ModelRef, args: WriteArgs) -> crate::IO<GraphqlId>;

    fn update_records(&self, model: ModelRef, where_: Filter, args: WriteArgs) -> crate::IO<Vec<GraphqlId>>;

    fn delete_records(&self, model: ModelRef, where_: Filter) -> crate::IO<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect(&self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId) -> crate::IO<()>;

    fn disconnect(&self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId) -> crate::IO<()>;

    fn set(&self, relation_field: RelationFieldRef, parent: GraphqlId, wheres: Vec<GraphqlId>) -> crate::IO<()>;
}
