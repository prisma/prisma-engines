use super::*;
use prisma_value::PrismaValue;

impl<'conn, 'tx> ReadOperations for ConnectionLike<'conn, 'tx> {
    fn get_single_record<'a>(
        &'a self,
        model: &'a ModelRef,
        filter: &'a Filter,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, Option<SingleRecord>> {
        match self {
            Self::Connection(c) => c.get_single_record(model, filter, selected_fields),
            Self::Transaction(tx) => tx.get_single_record(model, filter, selected_fields),
        }
    }

    fn get_many_records<'a>(
        &'a self,
        model: &'a ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a SelectedFields,
    ) -> crate::IO<'a, ManyRecords> {
        match self {
            Self::Connection(c) => c.get_many_records(model, query_arguments, selected_fields),
            Self::Transaction(tx) => tx.get_many_records(model, query_arguments, selected_fields),
        }
    }

    fn get_related_m2m_record_ids<'a>(
        &'a self,
        from_field: &'a RelationFieldRef,
        from_record_ids: &'a [RecordIdentifier],
    ) -> crate::IO<'a, Vec<(RecordIdentifier, RecordIdentifier)>> {
        match self {
            Self::Connection(c) => c.get_related_m2m_record_ids(from_field, from_record_ids),
            Self::Transaction(tx) => tx.get_related_m2m_record_ids(from_field, from_record_ids),
        }
    }

    // This will eventually become a more generic `aggregate`
    fn count_by_model<'a>(&'a self, model: &'a ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize> {
        match self {
            Self::Connection(c) => c.count_by_model(model, query_arguments),
            Self::Transaction(tx) => tx.count_by_model(model, query_arguments),
        }
    }
}

impl<'conn, 'tx> WriteOperations for ConnectionLike<'conn, 'tx> {
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> crate::IO<RecordIdentifier> {
        match self {
            Self::Connection(c) => c.create_record(model, args),
            Self::Transaction(tx) => tx.create_record(model, args),
        }
    }

    fn update_records<'a>(
        &'a self,
        model: &'a ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> crate::IO<Vec<RecordIdentifier>> {
        match self {
            Self::Connection(c) => c.update_records(model, where_, args),
            Self::Transaction(tx) => tx.update_records(model, where_, args),
        }
    }

    fn delete_records<'a>(&'a self, model: &'a ModelRef, where_: Filter) -> crate::IO<usize> {
        match self {
            Self::Connection(c) => c.delete_records(model, where_),
            Self::Transaction(tx) => tx.delete_records(model, where_),
        }
    }

    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordIdentifier,
        child_ids: &'a [RecordIdentifier],
    ) -> crate::IO<()> {
        match self {
            Self::Connection(c) => c.connect(field, parent_id, child_ids),
            Self::Transaction(tx) => tx.connect(field, parent_id, child_ids),
        }
    }

    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordIdentifier,
        child_ids: &'a [RecordIdentifier],
    ) -> crate::IO<()> {
        match self {
            Self::Connection(c) => c.disconnect(field, parent_id, child_ids),
            Self::Transaction(tx) => tx.disconnect(field, parent_id, child_ids),
        }
    }

    fn execute_raw<'a>(&'a self, query: String, parameters: Vec<PrismaValue>) -> crate::IO<serde_json::Value> {
        match self {
            Self::Connection(c) => c.execute_raw(query, parameters),
            Self::Transaction(tx) => tx.execute_raw(query, parameters),
        }
    }
}
