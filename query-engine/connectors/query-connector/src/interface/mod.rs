mod dispatch;

pub use dispatch::*;

use crate::{Filter, QueryArguments, WriteArgs};
use prisma_models::*;
use prisma_value::PrismaValue;

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
        model: &'a ModelRef,
        filter: &'a Filter,
        selected_fields: &'a ModelProjection,
    ) -> crate::IO<'a, Option<SingleRecord>>;

    fn get_many_records<'a>(
        &'a self,
        model: &'a ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a ModelProjection,
    ) -> crate::IO<'a, ManyRecords>;

    fn get_related_m2m_record_ids<'a>(
        &'a self,
        from_field: &'a RelationFieldRef,
        from_record_ids: &'a [RecordProjection],
    ) -> crate::IO<'a, Vec<(RecordProjection, RecordProjection)>>;

    // This will eventually become a more generic `aggregate`
    fn count_by_model<'a>(&'a self, model: &'a ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize>;
}

pub trait WriteOperations {
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> crate::IO<RecordProjection>;

    fn update_records<'a>(
        &'a self,
        model: &'a ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> crate::IO<Vec<RecordProjection>>;

    fn delete_records<'a>(&'a self, model: &'a ModelRef, where_: Filter) -> crate::IO<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordProjection,
        child_ids: &'a [RecordProjection],
    ) -> crate::IO<()>;

    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordProjection,
        child_ids: &'a [RecordProjection],
    ) -> crate::IO<()>;

    fn execute_raw<'a>(&'a self, query: String, parameters: Vec<PrismaValue>) -> crate::IO<serde_json::Value>;
}
