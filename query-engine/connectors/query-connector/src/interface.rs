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

pub enum ConnectionLike<'a> {
    Connection(&'a (dyn Connection + 'a)),
    Transaction(&'a (dyn Transaction<'a> + 'a))
}

impl<'a> ConnectionLike<'a> {
    pub fn as_read_operations(&self) -> &dyn ReadOperations {
        match self {
            Self::Connection(c) => c as &dyn ReadOperations,
            Self::Transaction(tx) => tx as &dyn ReadOperations,
        }
    }

    pub fn as_write_operations(&self) -> &dyn WriteOperations {
        match self {
            Self::Connection(c) => c,
            Self::Transaction(tx) => tx,
        }
    }
}

pub trait AllOperations<'a>: ReadOperations + WriteOperations + Send + Sync + 'a {}

pub trait ReadOperations {
    fn get_single_record<'a>(
        &'a self,
        record_finder: &'a RecordFinder,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, Option<SingleRecord>>;

    fn get_many_records<'a>(
        &'a self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords>;

    fn get_related_records<'a>(
        &'a self,
        from_field: RelationFieldRef,
        from_record_ids: &'a [GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords>;

    // This method is temporary
    fn get_scalar_list_values<'a>(
        &'a self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> crate::IO<'a, Vec<ScalarListValues>>;

    // This will eventually become a more generic `aggregate`
    fn count_by_model<'a>(&'a self, model: ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize>;
}

impl<'b> ReadOperations for ConnectionLike<'b> {
    fn get_single_record<'a>(
        &'a self,
        record_finder: &'a RecordFinder,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, Option<SingleRecord>> {
        self.as_read_operations().get_single_record(record_finder, selected_fields)
    }

    fn get_many_records<'a>(
        &'a self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords> {
        self.as_read_operations().get_many_records(model, query_arguments, selected_fields)
    }

    fn get_related_records<'a>(
        &'a self,
        from_field: RelationFieldRef,
        from_record_ids: &'a [GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords> {
        self.as_read_operations().get_related_records(from_field, from_record_ids, query_arguments, selected_fields)
    }

    // This method is temporary
    fn get_scalar_list_values<'a>(
        &'a self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> crate::IO<'a, Vec<ScalarListValues>> {
        self.as_read_operations().get_scalar_list_values(list_field, record_ids)
    }

    // This will eventually become a more generic `aggregate`
    fn count_by_model<'a>(&'a self, model: ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize> {
        self.as_read_operations().count_by_model(model, query_arguments)
    }
}

#[derive(Debug, Clone)]
pub struct ScalarListValues {
    pub record_id: GraphqlId,
    pub values: Vec<PrismaValue>,
}

pub trait WriteOperations {
    fn create_record<'a>(&'a self, model: ModelRef, args: WriteArgs) -> crate::IO<GraphqlId>;

    fn update_records<'a>(&'a self, model: ModelRef, where_: Filter, args: WriteArgs) -> crate::IO<Vec<GraphqlId>>;

    fn delete_records<'a>(&'a self, model: ModelRef, where_: Filter) -> crate::IO<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect<'a>(
        &'a self,
        field: RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()>;

    fn disconnect<'a>(
        &'a self,
        field: RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()>;

    fn set<'a>(
        &'a self,
        relation_field: RelationFieldRef,
        parent_id: GraphqlId,
        wheres: Vec<GraphqlId>,
    ) -> crate::IO<()>;
}

impl<'b> WriteOperations for ConnectionLike<'b> {
    fn create_record<'a>(&'a self, model: ModelRef, args: WriteArgs) -> crate::IO<GraphqlId> {
        self.as_write_operations().create_record(model, args)
    }

    fn update_records<'a>(&'a self, model: ModelRef, where_: Filter, args: WriteArgs) -> crate::IO<Vec<GraphqlId>> {
        self.as_write_operations().update_records(model, where_, args)
    }

    fn delete_records<'a>(&'a self, model: ModelRef, where_: Filter) -> crate::IO<usize> {
        self.as_write_operations().delete_records(model, where_)
    }

    fn connect<'a>(
        &'a self,
        field: RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()> {
        self.as_write_operations().connect(field, parent_id, child_id)
    }

    fn disconnect<'a>(
        &'a self,
        field: RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> crate::IO<()> {
        self.as_write_operations().disconnect(field, parent_id, child_id)
    }

    fn set<'a>(
        &'a self,
        relation_field: RelationFieldRef,
        parent_id: GraphqlId,
        wheres: Vec<GraphqlId>,
    ) -> crate::IO<()> {
        self.as_write_operations().set(relation_field, parent_id, wheres)
    }
}
