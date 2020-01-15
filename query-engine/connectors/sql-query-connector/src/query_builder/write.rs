use prisma_models::*;
use quaint::ast::*;
use connector_interface::{WriteArgs, FieldValueContainer};
use crate::error::SqlError;

const PARAMETER_LIMIT: usize = 10000;

pub fn create_record(model: &ModelRef, mut args: WriteArgs) -> (Insert<'static>, Option<RecordIdentifier>) {
    let return_id = args.as_record_identifier(model.identifier());

    let fields: Vec<_> = model
        .fields()
        .all
        .iter()
        .filter(|field| args.has_arg_for(&field.name()))
        .collect();

    let insert = fields
        .into_iter()
        .map(|field| (field.db_name(), args.take_field_value(field.name()).unwrap()))
        .fold(Insert::single_into(model.as_table()), |insert, (name, value)| match value {
            FieldValueContainer::Single(val) => insert.value(name.into_owned(), val),
            FieldValueContainer::Compound(_) => todo!("Compound schwompound"),
        });

    (
        Insert::from(insert).returning(model.identifier().as_columns()),
        return_id,
    )
}

pub fn delete_relation_table_records(
    field: &RelationFieldRef,
    parent_ids: &RecordIdentifier,
    child_ids: &[RecordIdentifier],
) -> Query<'static> {
    let relation = field.relation();
    let parent_columns: Vec<Column<'static>> = field.relation_columns(false).collect();

    let parent_ids: Vec<PrismaValue> = parent_ids.values().collect();
    let parent_id_criteria = Row::from(parent_columns).equals(parent_ids);

    let child_id_criteria = child_ids
        .into_iter()
        .map(|ids| {
            let cols_with_vals = field.opposite_columns(false).zip(ids.values());

            cols_with_vals.fold(ConditionTree::NoCondition, |acc, (col, val)| {
                match acc {
                    ConditionTree::NoCondition => col.equals(val).into(),
                    cond => cond.and(col.equals(val))
                }
            })
        }).fold(ConditionTree::NoCondition, |acc, cond| {
            match acc {
                ConditionTree::NoCondition => cond,
                acc => acc.or(cond),
            }
        });


    Delete::from_table(relation.as_table())
        .so_that(parent_id_criteria.and(child_id_criteria))
        .into()
}

pub fn delete_many(model: &ModelRef, ids: &[&RecordIdentifier]) -> Vec<Delete<'static>> {
    ids.chunks(PARAMETER_LIMIT).map(|chunk| {
        let condition = chunk.into_iter().map(|ids| {
            let cols_with_vals = model.identifier().as_columns().zip(ids.values());

            cols_with_vals.fold(ConditionTree::NoCondition, |acc, (col, val)| {
                match acc {
                    ConditionTree::NoCondition => col.equals(val).into(),
                    cond => cond.and(col.equals(val)),
                }
            })
        }).fold(ConditionTree::NoCondition, |acc, cond| {
            match acc {
                ConditionTree::NoCondition => cond,
                acc => acc.or(cond),
            }
        });

        Delete::from_table(model.as_table()).so_that(condition)
    }).collect()
}

pub fn create_relation_table_records(
    field: &RelationFieldRef,
    parent_id: &RecordIdentifier,
    child_ids: &[RecordIdentifier],
) -> Query<'static> {
    let relation = field.relation();
    let parent_columns = field.relation_columns(false).map(|c| c.name.to_string());
    let child_columns = field.opposite_columns(false).map(|c| c.name.to_string());

    let columns: Vec<String> = parent_columns.chain(child_columns).collect();
    let insert = Insert::multi_into(relation.as_table(), columns);

    let insert: MultiRowInsert = child_ids
        .into_iter()
        .fold(insert, |insert, child_id| {
            let values: Vec<_> = parent_id.values().chain(child_id.values()).collect();
            insert.values(values)
        })
        .into();

    insert.build().on_conflict(OnConflict::DoNothing).into()
}

pub fn update_many(model: &ModelRef, ids: &[&RecordIdentifier], args: &WriteArgs) -> crate::Result<Vec<Update<'static>>> {
    if args.args.is_empty() || ids.is_empty() {
        return Ok(Vec::new());
    }

    let fields = model.fields();
    let mut query = Update::table(model.as_table());

    for (name, value) in args.args.iter() {
        let field = fields.find_from_all(&name).unwrap();

        match value {
            FieldValueContainer::Single(value) => {
                if field.is_required() && value.is_null() {
                    return Err(SqlError::FieldCannotBeNull {
                        field: field.name().to_owned(),
                    });
                }

                query = query.set(field.db_name().to_string(), value.clone());
            }
            FieldValueContainer::Compound(_) => todo!("Not yet")
        }
    }

    let result: Vec<Update> = ids
        .chunks(PARAMETER_LIMIT)
        .into_iter()
        .map(|chunk| {
            let condition = chunk.into_iter().map(|ids| {
                let cols_with_vals = model.identifier().as_columns().zip(ids.values());

                cols_with_vals.fold(ConditionTree::NoCondition, |acc, (col, val)| {
                    match acc {
                        ConditionTree::NoCondition => col.equals(val).into(),
                        cond => cond.and(col.equals(val)),
                    }
                })
            }).fold(ConditionTree::NoCondition, |acc, cond| {
                match acc {
                    ConditionTree::NoCondition => cond,
                    acc => acc.or(cond),
                }
            });

            query
                .clone()
                .so_that(condition)
        })
        .collect();

    Ok(result)
}
