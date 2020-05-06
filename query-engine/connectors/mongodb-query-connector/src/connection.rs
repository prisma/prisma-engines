use connector_interface::filter::Filter;
use connector_interface::{self as connector, QueryArguments, ReadOperations, WriteArgs, WriteOperations};
use connector_interface::{Connection, RecordFilter, Transaction, IO};

use prisma_models::prelude::*;

/// A connection to a MongoDB database.
#[derive(Debug)]
pub struct MongodbConnection {}

impl Connection for MongodbConnection {
    fn start_transaction(&self) -> IO<'_, Box<dyn Transaction<'_> + '_>> {
        panic!("Mongodb transactions are not yet supported");
    }
}

impl ReadOperations for MongodbConnection {
    fn get_single_record(
        &self,
        _model: &ModelRef,
        _filter: &Filter,
        _selected_fields: &ModelProjection,
    ) -> connector::IO<'_, Option<SingleRecord>> {
        IO::new(async move {
            todo!();
        })
    }

    fn get_many_records(
        &self,
        _model: &ModelRef,
        _query_arguments: QueryArguments,
        _selected_fields: &ModelProjection,
    ) -> connector::IO<'_, ManyRecords> {
        IO::new(async move {
            todo!();
        })
    }

    fn get_related_m2m_record_ids(
        &self,
        _from_field: &RelationFieldRef,
        _from_record_ids: &[RecordProjection],
    ) -> connector::IO<'_, Vec<(RecordProjection, RecordProjection)>> {
        IO::new(async move {
            todo!();
        })
    }

    fn count_by_model(&self, _model: &ModelRef, _query_arguments: QueryArguments) -> connector::IO<'_, usize> {
        IO::new(async move {
            todo!();
        })
    }
}

impl WriteOperations for MongodbConnection {
    fn create_record(&self, _model: &ModelRef, _args: WriteArgs) -> connector::IO<'_, RecordProjection> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn update_records(
        &self,
        _model: &ModelRef,
        _record_filter: RecordFilter,
        _args: WriteArgs,
    ) -> connector::IO<'_, Vec<RecordProjection>> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn delete_records(&self, _model: &ModelRef, _record_filter: RecordFilter) -> connector::IO<'_, usize> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn connect(
        &self,
        _field: &RelationFieldRef,
        _parent_id: &RecordProjection,
        _child_ids: &[RecordProjection],
    ) -> connector::IO<'_, ()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn disconnect(
        &self,
        _field: &RelationFieldRef,
        _parent_id: &RecordProjection,
        _child_ids: &[RecordProjection],
    ) -> connector::IO<'_, ()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    fn execute_raw(&self, _query: String, _parameters: Vec<PrismaValue>) -> connector::IO<'_, serde_json::Value> {
        panic!("Write operations should be implemented on Transactions only");
    }
}
