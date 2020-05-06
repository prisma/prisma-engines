use connector_interface::filter::Filter;
use connector_interface::{self as connector, QueryArguments, ReadOperations, WriteArgs, WriteOperations};
use connector_interface::{Connection, RecordFilter, Transaction, IO};

use prisma_models::prelude::*;

/// A connection to a MongoDB database.
#[derive(Debug)]
pub struct MongodbConnection {}

impl Connection for MongodbConnection {
    fn start_transaction<'a>(&'a self) -> IO<'a, Box<dyn Transaction<'a> + 'a>> {
        panic!("Mongodb transactions are not yet supported");
    }
}

impl ReadOperations for MongodbConnection {
    fn get_single_record<'b>(
        &'b self,
        _model: &'b ModelRef,
        _filter: &'b Filter,
        _selected_fields: &'b ModelProjection,
    ) -> connector::IO<'b, Option<SingleRecord>> {
        IO::new(async move {
            todo!();
        })
    }

    fn get_many_records<'b>(
        &'b self,
        _model: &'b ModelRef,
        _query_arguments: QueryArguments,
        _selected_fields: &'b ModelProjection,
    ) -> connector::IO<'b, ManyRecords> {
        IO::new(async move {
            todo!();
        })
    }

    fn get_related_m2m_record_ids<'b>(
        &'b self,
        _from_field: &'b RelationFieldRef,
        _from_record_ids: &'b [RecordProjection],
    ) -> connector::IO<'b, Vec<(RecordProjection, RecordProjection)>> {
        IO::new(async move {
            todo!();
        })
    }

    fn count_by_model<'b>(
        &'b self,
        _model: &'b ModelRef,
        _query_arguments: QueryArguments,
    ) -> connector::IO<'b, usize> {
        IO::new(async move {
            todo!();
        })
    }
}

impl WriteOperations for MongodbConnection {
    fn create_record<'a>(&'a self, _model: &'a ModelRef, _args: WriteArgs) -> connector::IO<'_, RecordProjection> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn update_records<'b>(
        &'b self,
        _model: &'b ModelRef,
        _record_filter: RecordFilter,
        _args: WriteArgs,
    ) -> connector::IO<'b, Vec<RecordProjection>> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn delete_records<'b>(&'b self, _model: &'b ModelRef, _record_filter: RecordFilter) -> connector::IO<'b, usize> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn connect<'a>(
        &'a self,
        _field: &'a RelationFieldRef,
        _parent_id: &'a RecordProjection,
        _child_ids: &'a [RecordProjection],
    ) -> connector::IO<'_, ()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn disconnect<'a>(
        &'a self,
        _field: &'a RelationFieldRef,
        _parent_id: &'a RecordProjection,
        _child_ids: &'a [RecordProjection],
    ) -> connector::IO<'_, ()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn execute_raw<'a>(
        &'a self,
        _query: String,
        _parameters: Vec<PrismaValue>,
    ) -> connector::IO<'_, serde_json::Value> {
        panic!("Write operations should be implemented on Transactions only");
    }
}
