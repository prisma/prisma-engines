use super::*;
use async_trait::async_trait;
use prisma_value::PrismaValue;

#[async_trait]
impl<'conn, 'tx> ReadOperations for ConnectionLike<'conn, 'tx> {
    async fn get_single_record(
        &self,
        model: &ModelRef,
        filter: &Filter,
        selected_fields: &ModelProjection,
    ) -> crate::Result<Option<SingleRecord>> {
        match self {
            Self::Connection(c) => c.get_single_record(model, filter, selected_fields).await,
            Self::Transaction(tx) => tx.get_single_record(model, filter, selected_fields).await,
        }
    }

    async fn get_many_records(
        &self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &ModelProjection,
    ) -> crate::Result<ManyRecords> {
        match self {
            Self::Connection(c) => c.get_many_records(model, query_arguments, selected_fields).await,
            Self::Transaction(tx) => tx.get_many_records(model, query_arguments, selected_fields).await,
        }
    }

    async fn get_related_m2m_record_ids(
        &self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> crate::Result<Vec<(RecordProjection, RecordProjection)>> {
        match self {
            Self::Connection(c) => c.get_related_m2m_record_ids(from_field, from_record_ids).await,
            Self::Transaction(tx) => tx.get_related_m2m_record_ids(from_field, from_record_ids).await,
        }
    }

    async fn aggregate_records(
        &self,
        model: &ModelRef,
        aggregators: Vec<Aggregator>,
        query_arguments: QueryArguments,
    ) -> crate::Result<Vec<AggregationResult>> {
        match self {
            Self::Connection(c) => c.aggregate_records(model, aggregators, query_arguments).await,
            Self::Transaction(tx) => tx.aggregate_records(model, aggregators, query_arguments).await,
        }
    }
}

#[async_trait]
impl<'conn, 'tx> WriteOperations for ConnectionLike<'conn, 'tx> {
    async fn create_record(&self, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordProjection> {
        match self {
            Self::Connection(c) => c.create_record(model, args).await,
            Self::Transaction(tx) => tx.create_record(model, args).await,
        }
    }

    async fn update_records(
        &self,
        model: &ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
    ) -> crate::Result<Vec<RecordProjection>> {
        match self {
            Self::Connection(c) => c.update_records(model, record_filter, args).await,
            Self::Transaction(tx) => tx.update_records(model, record_filter, args).await,
        }
    }

    async fn delete_records(&self, model: &ModelRef, record_filter: RecordFilter) -> crate::Result<usize> {
        match self {
            Self::Connection(c) => c.delete_records(model, record_filter).await,
            Self::Transaction(tx) => tx.delete_records(model, record_filter).await,
        }
    }

    async fn connect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> crate::Result<()> {
        match self {
            Self::Connection(c) => c.connect(field, parent_id, child_ids).await,
            Self::Transaction(tx) => tx.connect(field, parent_id, child_ids).await,
        }
    }

    async fn disconnect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> crate::Result<()> {
        match self {
            Self::Connection(c) => c.disconnect(field, parent_id, child_ids).await,
            Self::Transaction(tx) => tx.disconnect(field, parent_id, child_ids).await,
        }
    }

    async fn query_raw(&self, query: String, parameters: Vec<PrismaValue>) -> crate::Result<serde_json::Value> {
        match self {
            Self::Connection(c) => c.query_raw(query, parameters).await,
            Self::Transaction(tx) => tx.query_raw(query, parameters).await,
        }
    }

    async fn execute_raw(&self, query: String, parameters: Vec<PrismaValue>) -> crate::Result<usize> {
        match self {
            Self::Connection(c) => c.execute_raw(query, parameters).await,
            Self::Transaction(tx) => tx.execute_raw(query, parameters).await,
        }
    }
}
