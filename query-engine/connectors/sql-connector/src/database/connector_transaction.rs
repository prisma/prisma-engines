use prisma_query::connector::Queryable;

use connector_interface::*;
use prisma_models::*;

use crate::query_builder::write::NestedActions;
use crate::query_builder::{ManyRelatedRecordsWithUnionAll, WriteQueryBuilder};
use crate::transactional;
use crate::SqlError;

pub struct ConnectorTransaction<'a> {
    inner: prisma_query::connector::Transaction<'a>,
}

impl<'a> ConnectorTransaction<'a> {
    pub fn new(tx: prisma_query::connector::Transaction) -> ConnectorTransaction {
        ConnectorTransaction { inner: tx }
    }

    pub fn commit(self) -> crate::Result<()> {
        Ok(self.inner.commit().map_err(SqlError::from)?)
    }
}

impl MaybeTransaction for ConnectorTransaction<'_> {}

impl ReadOperations for ConnectorTransaction<'_> {
    fn get_single_record(
        &mut self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        let result = transactional::execute_get_single_record(&mut self.inner, record_finder, selected_fields)?;
        Ok(result)
    }

    fn get_many_records(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        let result = transactional::execute_get_many_records(&mut self.inner, model, query_arguments, selected_fields)?;
        Ok(result)
    }

    fn get_related_records(
        &mut self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector_interface::Result<ManyRecords> {
        // TODO: we must pass the right related records builder as a type parameter.
        let result = transactional::execute_get_related_records::<ManyRelatedRecordsWithUnionAll>(
            &mut self.inner,
            from_field,
            from_record_ids,
            query_arguments,
            selected_fields,
        )?;
        Ok(result)
    }

    fn count_by_model(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
    ) -> connector_interface::Result<usize> {
        let result = transactional::execute_count_by_model(&mut self.inner, model, query_arguments)?;
        Ok(result)
    }
}
impl WriteOperations for ConnectorTransaction<'_> {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> connector_interface::Result<GraphqlId> {
        let result = transactional::create::execute(&mut self.inner, model, args.non_list_args(), args.list_args())?;
        Ok(result)
    }

    fn update_records(
        &mut self,
        model: ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector_interface::Result<usize> {
        let result = transactional::update_many::execute(
            &mut self.inner,
            model,
            &where_,
            args.non_list_args(),
            args.list_args(),
        )?;
        Ok(result)
    }

    fn delete_records(&mut self, model: ModelRef, where_: Filter) -> connector_interface::Result<usize> {
        let result = transactional::delete_many::execute(&mut self.inner, model, &where_)?;
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
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector_interface::Result<()> {
        let child_model = field.related_model();

        let nested_disconnect = NestedDisconnect {
            relation_field: field,
            where_: Some(RecordFinder::new(child_model.fields().id(), child_id)),
        };

        let query = nested_disconnect.removal_by_parent_and_child(parent_id, child_id);
        self.inner.execute(query).unwrap();

        Ok(())
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
