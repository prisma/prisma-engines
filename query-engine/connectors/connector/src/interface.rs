use crate::{Filter, QueryArguments, RecordFinder, WriteArgs};
use prisma_models::*;

pub trait Connector {
    fn with_transaction<F, T>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn TransactionLike) -> crate::Result<T>;
}

pub trait TransactionLike: ReadOperations + WriteOperations {}

// AdditionalOperations? MetaOperations? ...
// pub trait UtilityOperations {
//     fn execute_raw(&self, db_name: String, query: String) -> crate::Result<Value>;
//     fn execute_raw(&self, db_name: String, query: String) -> connector_interface::Result<Value> {
//         let result = self.inner.raw_json(RawQuery::from(query))?;
//         Ok(result)
//     }

//     reset data
// }

pub trait ReadOperations {
    fn get_single_record(
        &mut self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> crate::Result<Option<SingleRecord>>;

    fn get_many_records(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> crate::Result<ManyRecords>;

    fn get_related_records(
        &mut self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> crate::Result<ManyRecords>;

    // This method is temporary
    fn get_scalar_list_values(
        &mut self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> crate::Result<Vec<ScalarListValues>>;

    // This will eventually become a more generic `aggregate`
    fn count_by_model(&mut self, model: ModelRef, query_arguments: QueryArguments) -> crate::Result<usize>;
}

#[derive(Debug, Clone)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}

pub trait WriteOperations {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> crate::Result<GraphqlId>;

    fn update_records(&mut self, model: ModelRef, where_: Filter, args: WriteArgs) -> crate::Result<usize>;

    fn delete_records(&mut self, model: ModelRef, where_: Filter) -> crate::Result<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId) -> crate::Result<()>;

    fn disconnect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId)
        -> crate::Result<()>;

    fn set(&mut self, relation_field: RelationFieldRef, parent: GraphqlId, wheres: Vec<GraphqlId>)
        -> crate::Result<()>;
}
