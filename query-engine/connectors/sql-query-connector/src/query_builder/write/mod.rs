use crate::error::SqlError;
use prisma_models::*;
use quaint::ast::*;
use std::convert::TryFrom;

const PARAMETER_LIMIT: usize = 10000;

pub fn create_record(model: &ModelRef, mut args: PrismaArgs) -> (Insert<'static>, Option<GraphqlId>) {
    let id_field = model.fields().id();

    let return_id = match args.get_field_value(&id_field.name) {
        _ if id_field.is_auto_generated => None,
        Some(PrismaValue::Null) | None => {
            let id = model.generate_id();
            args.insert(id_field.name.as_str(), id.clone());
            Some(id)
        }
        Some(prisma_value) => {
            Some(GraphqlId::try_from(prisma_value).expect("Could not convert prisma value to graphqlid"))
        }
    };

    let fields: Vec<&Field> = model
        .fields()
        .all
        .iter()
        .filter(|field| args.has_arg_for(&field.name()))
        .collect();

    let fields = fields
        .iter()
        .map(|field| (field.db_name(), args.take_field_value(field.name()).unwrap()));

    let base = Insert::single_into(model.as_table());

    let insert = fields
        .into_iter()
        .fold(base, |acc, (name, value)| acc.value(name.into_owned(), value));

    (Insert::from(insert).returning(vec![id_field.as_column()]), return_id)
}

pub fn create_relation_table_records(
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> Query<'static> {
    let relation = field.relation();
    let parent_column = field.relation_column();
    let child_column = field.opposite_column();

    let mut columns = vec![parent_column.name.to_string(), child_column.name.to_string()];
    if let Some(id_col) = relation.id_column() {
        columns.push(id_col.name.to_string());
    };

    let generate_ids = relation.id_column().is_some();
    let insert = Insert::multi_into(relation.as_table(), columns);
    let insert: MultiRowInsert = child_ids
        .into_iter()
        .fold(insert, |insert, child_id| {
            if generate_ids {
                insert.values((parent_id.clone(), child_id.clone(), cuid::cuid().unwrap()))
            } else {
                insert.values((parent_id.clone(), child_id.clone()))
            }
        })
        .into();

    insert.build().on_conflict(OnConflict::DoNothing).into()
}

pub fn delete_relation_table_records(
    field: &RelationFieldRef,
    parent_id: &GraphqlId,
    child_ids: &[GraphqlId],
) -> Query<'static> {
    let relation = field.relation();
    let parent_column = field.relation_column();
    let child_column = field.opposite_column();

    let parent_id_criteria = parent_column.equals(parent_id);
    let child_id_criteria = child_column.in_selection(child_ids.to_owned());

    Delete::from_table(relation.as_table())
        .so_that(parent_id_criteria.and(child_id_criteria))
        .into()
}

pub fn update_many(model: &ModelRef, ids: &[&GraphqlId], args: &PrismaArgs) -> crate::Result<Vec<Update<'static>>> {
    if args.args.is_empty() || ids.is_empty() {
        return Ok(Vec::new());
    }

    let fields = model.fields();
    let mut query = Update::table(model.as_table());

    for (name, value) in args.args.iter() {
        let field = fields.find_from_all(&name).unwrap();

        if field.is_required() && value.is_null() {
            return Err(SqlError::FieldCannotBeNull {
                field: field.name().to_owned(),
            });
        }

        query = query.set(field.db_name().to_string(), value.clone());
    }

    let result: Vec<Update> = ids
        .chunks(PARAMETER_LIMIT)
        .into_iter()
        .map(|ids| {
            query
                .clone()
                .so_that(fields.id().as_column().in_selection(ids.to_vec()))
        })
        .collect();

    Ok(result)
}

pub fn delete_many(model: &ModelRef, ids: &[&GraphqlId]) -> Vec<Delete<'static>> {
    let mut deletes = Vec::new();

    for chunk in ids.chunks(PARAMETER_LIMIT).into_iter() {
        for lf in model.fields().scalar_list() {
            let scalar_list_table = lf.scalar_list_table();
            let condition = scalar_list_table.node_id_column().in_selection(chunk.to_vec());
            deletes.push(Delete::from_table(scalar_list_table.table()).so_that(condition));
        }

        let condition = model.fields().id().as_column().in_selection(chunk.to_vec());
        deletes.push(Delete::from_table(model.as_table()).so_that(condition));
    }

    deletes
}

fn delete_in_chunks<F>(table: Table<'static>, ids: &[&GraphqlId], conditions: F) -> Vec<Delete<'static>>
where
    F: Fn(&[&GraphqlId]) -> Compare<'static>,
{
    ids.chunks(PARAMETER_LIMIT)
        .into_iter()
        .map(|chunk| Delete::from_table(table.clone()).so_that(conditions(chunk)))
        .collect()
}
