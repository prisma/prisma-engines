use crate::query_builder::read::ManyRelatedRecordsQueryBuilder;
use crate::{operations, query_builder::WriteQueryBuilder, SqlError};
use connector_interface::*;
use prisma_models::*;
use prisma_query::connector::Queryable;
use std::marker::PhantomData;

pub struct SqlConnectorTransaction<'a, T> {
    inner: prisma_query::connector::Transaction<'a>,
    _p: PhantomData<T>,
}

impl<'a, T> SqlConnectorTransaction<'a, T> {
    pub fn new(tx: prisma_query::connector::Transaction<'a>) -> Self {
        Self {
            inner: tx,
            _p: PhantomData,
        }
    }

    pub fn commit(self) -> connector_interface::Result<()> {
        Ok(self.inner.commit().map_err(SqlError::from)?)
    }
}

impl<T> TransactionLike for SqlConnectorTransaction<'_, T> where T: ManyRelatedRecordsQueryBuilder {}

impl<T> ReadOperations for SqlConnectorTransaction<'_, T>
where
    T: ManyRelatedRecordsQueryBuilder,
{
    fn get_single_record(
        &mut self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        let result = operations::execute_get_single_record(&mut self.inner, record_finder, selected_fields)?;
        Ok(result)
    }

    fn get_many_records(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        let result = operations::execute_get_many_records(&mut self.inner, model, query_arguments, selected_fields)?;
        Ok(result)
    }

    fn get_related_records(
        &mut self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        let result = operations::execute_get_related_records::<T>(
            &mut self.inner,
            from_field,
            from_record_ids,
            query_arguments,
            selected_fields,
        )?;

        Ok(result)
    }

    fn get_scalar_list_values(
        &mut self,
        _list_field: ScalarFieldRef,
        _record_ids: Vec<GraphqlId>,
    ) -> connector_interface::Result<Vec<ScalarListValues>> {
        unimplemented!()
        //get_scalar_list_values_by_record_ids
    }

    fn count_by_model(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
    ) -> connector_interface::Result<usize> {
        let result = operations::execute_count_by_model(&mut self.inner, model, query_arguments)?;
        Ok(result)
    }
}

impl<T> WriteOperations for SqlConnectorTransaction<'_, T> {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> connector_interface::Result<GraphqlId> {
        let result = operations::create::execute(&mut self.inner, model, args.non_list_args(), args.list_args())?;
        Ok(result)
    }

    fn update_records(
        &mut self,
        model: ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector_interface::Result<usize> {
        let result =
            operations::update_many::execute(&mut self.inner, model, &where_, args.non_list_args(), args.list_args())?;

        Ok(result)
    }

    fn delete_records(&mut self, model: ModelRef, where_: Filter) -> connector_interface::Result<usize> {
        let result = operations::delete_many::execute(&mut self.inner, model, &where_)?;
        Ok(result)
    }

    fn connect(
        &mut self,
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector_interface::Result<()> {
        let query = WriteQueryBuilder::create_relation(field, parent_id, child_id);

        self.inner.execute(query).unwrap();
        Ok(())
    }

    fn disconnect(
        &mut self,
        _field: RelationFieldRef,
        _parent_id: &GraphqlId,
        _child_id: &GraphqlId,
    ) -> connector_interface::Result<()> {
        // let child_model = field.related_model();

        // let nested_disconnect = NestedDisconnect {
        //     relation_field: field,
        //     where_: Some(RecordFinder::new(child_model.fields().id(), child_id)),
        // };

        // let query = nested_disconnect.removal_by_parent_and_child(parent_id, child_id);
        // self.inner.execute(query).unwrap();

        // Ok(())
        unimplemented!()
    }

    fn set(
        &mut self,
        _relation_field: RelationFieldRef,
        _parent: GraphqlId,
        _wheres: Vec<GraphqlId>,
    ) -> connector_interface::Result<()> {
        unimplemented!()
    }
}
