use connector_interface::WriteArgs;
use prisma_models::*;
use quaint::ast::*;

pub fn create_record(model: &ModelRef, mut args: WriteArgs) -> (Insert<'static>, Option<RecordIdentifier>) {
    let return_id = args.as_record_identifier(model.primary_identifier());

    let fields: Vec<_> = model
        .fields()
        .db_names()
        .filter_map(|db_name| {
            if args.has_arg_for(&db_name) {
                Some(db_name)
            } else {
                None
            }
        })
        .collect();

    let insert = fields
        .into_iter()
        .fold(Insert::single_into(model.as_table()), |insert, db_name| {
            let value = args.take_field_value(&db_name).unwrap();
            insert.value(db_name, value)
        });

    (
        Insert::from(insert).returning(model.primary_identifier().as_columns()),
        return_id,
    )
}

pub fn delete_relation_table_records(
    field: &RelationFieldRef,
    parent_ids: &RecordIdentifier,
    child_ids: &[RecordIdentifier],
) -> Delete<'static> {
    let columns: Vec<_> = field.opposite_columns(false).collect();

    let relation = field.relation();
    let parent_columns: Vec<Column<'static>> = field.relation_columns(false).collect();

    let parent_ids: Vec<PrismaValue> = parent_ids.values().collect();
    let parent_id_criteria = Row::from(parent_columns).equals(parent_ids);

    let child_id_criteria = super::conditions(&columns, child_ids);

    Delete::from_table(relation.as_table()).so_that(parent_id_criteria.and(child_id_criteria))
}

pub fn delete_many(model: &ModelRef, ids: &[&RecordIdentifier]) -> Vec<Query<'static>> {
    let columns: Vec<_> = model.primary_identifier().as_columns().collect();

    super::chunked_conditions(&columns, ids, |conditions| {
        Delete::from_table(model.as_table()).so_that(conditions)
    })
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

pub fn update_many(model: &ModelRef, ids: &[&RecordIdentifier], args: WriteArgs) -> crate::Result<Vec<Query<'static>>> {
    if args.args.is_empty() || ids.is_empty() {
        return Ok(Vec::new());
    }

    let query = args
        .args
        .into_iter()
        .fold(Update::table(model.as_table()), |acc, (name, val)| {
            acc.set(name, val.clone())
        });

    let columns: Vec<_> = model.primary_identifier().as_columns().collect();
    let result: Vec<Query> = super::chunked_conditions(&columns, ids, |conditions| query.clone().so_that(conditions));

    Ok(result)
}
