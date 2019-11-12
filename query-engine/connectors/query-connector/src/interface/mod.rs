mod dispatch;

pub use dispatch::*;

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

pub enum ConnectionLike<'conn, 'tx>
where
    'tx: 'conn,
{
    Connection(&'conn (dyn Connection + 'conn)),
    Transaction(&'conn (dyn Transaction<'tx> + 'tx)),
}

pub trait ReadOperations {
    fn get_single_record<'a>(
        &'a self,
        record_finder: &'a RecordFinder,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, Option<SingleRecord>>;

    fn get_many_records<'a>(
        &'a self,
        model: &'a ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords>;

    fn get_related_records<'a>(
        &'a self,
        from_field: &'a RelationFieldRef,
        from_record_ids: &'a [GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords>;

    // This method is temporary
    fn get_scalar_list_values<'a>(
        &'a self,
        list_field: &'a ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> crate::IO<'a, Vec<ScalarListValues>>;

    // This will eventually become a more generic `aggregate`
    fn count_by_model<'a>(&'a self, model: &'a ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize>;
}

#[derive(Debug, Clone)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}

pub trait WriteOperations {
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> crate::IO<GraphqlId>;

    fn update_records<'a>(&'a self, model: &'a ModelRef, where_: Filter, args: WriteArgs) -> crate::IO<Vec<GraphqlId>>;

    fn delete_records<'a>(&'a self, model: &'a ModelRef, where_: Filter) -> crate::IO<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()>;

    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()>;

    fn set<'a>(
        &'a self,
        relation_field: &'a RelationFieldRef,
        parent_id: GraphqlId,
        wheres: Vec<GraphqlId>,
    ) -> crate::IO<()>;
}
