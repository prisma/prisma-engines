use connector_interface::{DatasourceFieldName, WriteArgs, WriteExpression};
use prisma_models::*;
use quaint::ast::*;
use std::convert::TryInto;

/// `INSERT` a new record to the database. Resulting an `INSERT` ast and an
/// optional `RecordProjection` if available from the arguments or model.
pub fn create_record(model: &ModelRef, mut args: WriteArgs) -> (Insert<'static>, Option<RecordProjection>) {
    let return_id = args.as_record_projection(model.primary_identifier());

    let fields: Vec<_> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|field| args.has_arg_for(&field.db_name()))
        .collect();

    let insert = fields
        .into_iter()
        .fold(Insert::single_into(model.as_table()), |insert, field| {
            let db_name = field.db_name();
            let value = args.take_field_value(db_name).unwrap();
            let value: PrismaValue = value
                .try_into()
                .expect("Create calls can only use PrismaValue write expressions (right now).");

            insert.value(db_name.to_owned(), field.value(value))
        });

    (
        Insert::from(insert).returning(model.primary_identifier().as_columns()),
        return_id,
    )
}

pub fn update_many(model: &ModelRef, ids: &[&RecordProjection], args: WriteArgs) -> crate::Result<Vec<Query<'static>>> {
    if args.args.is_empty() || ids.is_empty() {
        return Ok(Vec::new());
    }

    let scalar_fields = model.fields().scalar();

    let query = args
        .args
        .into_iter()
        .fold(Update::table(model.as_table()), |acc, (field_name, val)| {
            let DatasourceFieldName(name) = field_name;
            let field = scalar_fields
                .iter()
                .find(|f| f.db_name() == &name)
                .expect("Expected field to be valid");

            let value: Expression = match val {
                WriteExpression::Field(_) => unimplemented!(),
                WriteExpression::Value(rhs) => field.value(rhs).into(),
                WriteExpression::Add(rhs) => {
                    let e: Expression<'_> = Column::from(name.clone()).into();
                    e + field.value(rhs).into()
                }

                WriteExpression::Substract(rhs) => {
                    let e: Expression<'_> = Column::from(name.clone()).into();
                    e - field.value(rhs).into()
                }

                WriteExpression::Multiply(rhs) => {
                    let e: Expression<'_> = Column::from(name.clone()).into();
                    e * field.value(rhs).into()
                }

                WriteExpression::Divide(rhs) => {
                    let e: Expression<'_> = Column::from(name.clone()).into();
                    e / field.value(rhs).into()
                }
            };

            acc.set(name, value)
        });

    let columns: Vec<_> = model.primary_identifier().as_columns().collect();
    let result: Vec<Query> = super::chunked_conditions(&columns, ids, |conditions| query.clone().so_that(conditions));

    Ok(result)
}

pub fn delete_many(model: &ModelRef, ids: &[&RecordProjection]) -> Vec<Query<'static>> {
    let columns: Vec<_> = model.primary_identifier().as_columns().collect();

    super::chunked_conditions(&columns, ids, |conditions| {
        Delete::from_table(model.as_table()).so_that(conditions)
    })
}

pub fn create_relation_table_records(
    field: &RelationFieldRef,
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> Query<'static> {
    let relation = field.relation();
    let parent_columns: Vec<_> = field.related_field().m2m_columns();
    let child_columns: Vec<_> = field.m2m_columns();

    let columns: Vec<_> = parent_columns.into_iter().chain(child_columns).collect();
    let insert = Insert::multi_into(relation.as_table(), columns);

    let insert: MultiRowInsert = child_ids.iter().fold(insert, |insert, child_id| {
        let mut values: Vec<_> = parent_id.db_values();

        values.extend(child_id.db_values());
        insert.values(values)
    });

    insert.build().on_conflict(OnConflict::DoNothing).into()
}

pub fn delete_relation_table_records(
    parent_field: &RelationFieldRef,
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> Delete<'static> {
    let relation = parent_field.relation();

    let mut parent_columns: Vec<_> = parent_field.related_field().m2m_columns();
    let child_columns: Vec<_> = parent_field.m2m_columns();

    let parent_id_values = parent_id.db_values();
    let parent_id_criteria = if parent_columns.len() > 1 {
        Row::from(parent_columns).equals(parent_id_values)
    } else {
        parent_columns.pop().unwrap().equals(parent_id_values)
    };

    let child_id_criteria = super::conditions(&child_columns, child_ids);

    Delete::from_table(relation.as_table()).so_that(parent_id_criteria.and(child_id_criteria))
}
