use crate::{
    error::SqlError,
    query_builder::{WriteQueryBuilder},
    Transaction,
};
use prisma_models::{GraphqlId, ModelRef, PrismaArgs, PrismaListValue, RelationFieldRef};
use prisma_query::error::Error as QueryError;
use std::sync::Arc;

/// Creates a new root record and any associated list records to the database.
pub fn execute<S>(
    conn: &mut dyn Transaction,
    model: ModelRef,
    non_list_args: &PrismaArgs,
    list_args: &[(S, PrismaListValue)],
) -> crate::Result<GraphqlId>
where
    S: AsRef<str>,
{
    let (insert, returned_id) = WriteQueryBuilder::create_record(Arc::clone(&model), non_list_args.clone());

    let last_id = match conn.insert(insert) {
        Ok(id) => id,
        Err(QueryError::UniqueConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(SqlError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(SqlError::UniqueConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(QueryError::NullConstraintViolation { field_name }) => {
            if field_name == "PRIMARY" {
                return Err(SqlError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, model.fields().id().name),
                });
            } else {
                return Err(SqlError::NullConstraintViolation {
                    field_name: format!("{}.{}", model.name, field_name),
                });
            }
        }
        Err(e) => return Err(SqlError::from(e)),
    };

    let id = match returned_id {
        Some(id) => id,
        None => GraphqlId::from(last_id.unwrap()),
    };

    for (field_name, list_value) in list_args {
        let field = model.fields().find_from_scalar(field_name.as_ref()).unwrap();
        let table = field.scalar_list_table();

        if let Some(insert) = WriteQueryBuilder::create_scalar_list_value(table.table(), &list_value, &id) {
            conn.insert(insert)?;
        }
    }

    Ok(id)
}