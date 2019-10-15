use super::SqlConnectorTransaction;
use crate::{
    error::SqlError,
    query_builder::{DeleteActions, WriteQueryBuilder},
    QueryExt,
};
use connector_interface::{error::ConnectorError, *};
use prisma_models::*;
use prisma_query::{connector::Queryable, error::Error as QueryError};
use std::sync::Arc;

impl<T> WriteOperations for SqlConnectorTransaction<'_, T> {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> connector_interface::Result<GraphqlId> {
        let (insert, returned_id) = WriteQueryBuilder::create_record(Arc::clone(&model), args.non_list_args().clone());

        let last_id = match self.inner.insert(insert) {
            Ok(id) => id,
            Err(QueryError::UniqueConstraintViolation { field_name }) => {
                if field_name == "PRIMARY" {
                    return Err(ConnectorError::UniqueConstraintViolation {
                        field_name: format!("{}.{}", model.name, model.fields().id().name),
                    });
                } else {
                    return Err(ConnectorError::UniqueConstraintViolation {
                        field_name: format!("{}.{}", model.name, field_name),
                    });
                }
            }
            Err(QueryError::NullConstraintViolation { field_name }) => {
                if field_name == "PRIMARY" {
                    return Err(ConnectorError::NullConstraintViolation {
                        field_name: format!("{}.{}", model.name, model.fields().id().name),
                    });
                } else {
                    return Err(ConnectorError::NullConstraintViolation {
                        field_name: format!("{}.{}", model.name, field_name),
                    });
                }
            }
            Err(e) => return Err(SqlError::from(e).into()),
        };

        let id = match returned_id {
            Some(id) => id,
            None => GraphqlId::from(last_id.unwrap()),
        };

        for (field_name, list_value) in args.list_args() {
            let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
            let table = field.scalar_list_table();

            if let Some(insert) = WriteQueryBuilder::create_scalar_list_value(table.table(), &list_value, &id) {
                self.inner.insert(insert).map_err(SqlError::from)?;
            }
        }

        Ok(id)
    }

    fn update_records(
        &mut self,
        model: ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector_interface::Result<Vec<GraphqlId>> {
        let ids = self.inner.filter_ids(Arc::clone(&model), where_.clone())?;

        if ids.len() == 0 {
            return Ok(vec![]);
        }

        let updates = {
            let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
            WriteQueryBuilder::update_many(Arc::clone(&model), ids.as_slice(), args.non_list_args())?
        };

        for update in updates {
            self.inner.update(update).map_err(SqlError::from)?;
        }

        for (field_name, list_value) in args.list_args() {
            let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
            let table = field.scalar_list_table();
            let (deletes, inserts) = WriteQueryBuilder::update_scalar_list_values(&table, &list_value, ids.to_vec());

            for delete in deletes {
                self.inner.delete(delete).map_err(SqlError::from)?;
            }

            for insert in inserts {
                self.inner.insert(insert).map_err(SqlError::from)?;
            }
        }

        Ok(ids)
    }

    fn delete_records(&mut self, model: ModelRef, where_: Filter) -> connector_interface::Result<usize> {
        let ids = self.inner.filter_ids(Arc::clone(&model), where_.clone())?;
        let ids: Vec<&GraphqlId> = ids.iter().map(|id| &*id).collect();
        let count = ids.len();

        if count == 0 {
            return Ok(count);
        }

        DeleteActions::check_relation_violations(Arc::clone(&model), ids.as_slice(), |select| {
            let ids = self.inner.select_ids(select)?;
            Ok(ids.into_iter().next())
        })?;

        for delete in WriteQueryBuilder::delete_many(model, ids.as_slice()) {
            self.inner.delete(delete).map_err(SqlError::from)?;
        }

        Ok(count)
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
        let query = WriteQueryBuilder::delete_relation(field, parent_id, child_id);
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
