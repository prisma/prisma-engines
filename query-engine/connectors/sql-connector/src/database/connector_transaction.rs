use prisma_query::connector::Queryable;

use connector_interface::*;
use prisma_models::*;

use crate::query_builder::write::NestedActions;
use crate::query_builder::WriteQueryBuilder;
use crate::transactional::create;
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

impl ReadOperations for ConnectorTransaction<'_> {}
impl WriteOperations for ConnectorTransaction<'_> {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> connector_interface::Result<GraphqlId> {
        let result = create::execute(&mut self.inner, model, args.non_list_args(), args.list_args())?;
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
