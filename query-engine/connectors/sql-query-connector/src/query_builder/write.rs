use prisma_models::*;
use quaint::ast::*;
use connector_interface::WriteArgs;

// const PARAMETER_LIMIT: usize = 10000;

pub fn create_record(_model: &ModelRef, _args: WriteArgs) -> (Insert<'static>, Option<GraphqlId>) {
    // let mut id_fields = model.primary_identifier();
    // let return_id = args
    //     .get_field_value(&id_field.name)
    //     .map(|id| GraphqlId::try_from(id).expect("Could not convert prisma value to graphqlid"));

    // // --------------------- Snippets
    // // let id_field = if id_fields.len() == 1 {
    // //     id_fields.pop().unwrap()
    // // } else {
    // //     panic!("Multi-field IDs are not (yet) supported.")
    // // };

    // // let return_id = match args.get_field_value(&id_field.name) {
    // //     _ if id_field.is_auto_generated => None,

    // //     Some(PrismaValue::Null) | None => {
    // //         let id = model.generate_id();
    // //         args.insert(id_field.name.as_str(), id.clone());
    // //         Some(id)
    // //     }

    // //     Some(prisma_value) => {
    // //         Some(GraphqlId::try_from(prisma_value).expect("Could not convert prisma value to graphqlid"))
    // //     }
    // // };
    // // ---------------------

    // let fields: Vec<&Field> = model
    //     .fields()
    //     .all
    //     .iter()
    //     .filter(|field| args.has_arg_for(&field.name()))
    //     .collect();

    // let fields = fields
    //     .iter()
    //     .map(|field| (field.db_name(), args.take_field_value(field.name()).unwrap()));

    // let base = Insert::single_into(model.as_table());

    // let insert = fields
    //     .into_iter()
    //     .fold(base, |acc, (name, value)| acc.value(name.into_owned(), value));

    // (Insert::from(insert).returning(vec![id_field.as_column()]), return_id)

    todo!()
}

/*
pub fn create_relation_table_records(
    _field: &RelationFieldRef,
    _parent_id: &GraphqlId,
    _child_ids: &[GraphqlId],
) -> Query<'static> {
    // let relation = field.relation();
    // let parent_column = field.relation_column(false);
    // let child_column = field.opposite_column(false);

    // let mut columns = vec![parent_column.name.to_string(), child_column.name.to_string()];
    // if let Some(id_col) = relation.id_column() {
    //     columns.push(id_col.name.to_string());
    // };

    // let generate_ids = relation.id_column().is_some();
    // let insert = Insert::multi_into(relation.as_table(), columns);
    // let insert: MultiRowInsert = child_ids
    //     .into_iter()
    //     .fold(insert, |insert, child_id| {
    //         if generate_ids {
    //             insert.values((parent_id.clone(), child_id.clone(), cuid::cuid().unwrap()))
    //         } else {
    //             insert.values((parent_id.clone(), child_id.clone()))
    //         }
    //     })
    //     .into();

    // insert.build().on_conflict(OnConflict::DoNothing).into()

    todo!()
}

pub fn delete_relation_table_records(
    _field: &RelationFieldRef,
    _parent_id: &GraphqlId,
    _child_ids: &[GraphqlId],
) -> Query<'static> {
    // let relation = field.relation();
    // let parent_column = field.relation_column(false);
    // let child_column = field.opposite_column(false);

    // let parent_id_criteria = parent_column.equals(parent_id);
    // let child_id_criteria = child_column.in_selection(child_ids.to_owned());

    // Delete::from_table(relation.as_table())
    //     .so_that(parent_id_criteria.and(child_id_criteria))
    //     .into()

    todo!()
}

pub fn update_many(_model: &ModelRef, _ids: &[&GraphqlId], _args: &WriteArgs) -> crate::Result<Vec<Update<'static>>> {
    // if args.args.is_empty() || ids.is_empty() {
    //     return Ok(Vec::new());
    // }

    // let fields = model.fields();
    // let mut query = Update::table(model.as_table());

    // for (name, value) in args.args.iter() {
    //     let field = fields.find_from_all(&name).unwrap();

    //     if field.is_required() && value.is_null() {
    //         return Err(SqlError::FieldCannotBeNull {
    //             field: field.name().to_owned(),
    //         });
    //     }

    //     query = query.set(field.db_name().to_string(), value.clone());
    // }

    // let result: Vec<Update> = ids
    //     .chunks(PARAMETER_LIMIT)
    //     .into_iter()
    //     .map(|ids| {
    //         query
    //             .clone()
    //             .so_that(fields.id().as_column().in_selection(ids.to_vec()))
    //     })
    //     .collect();

    // Ok(result)

    todo!()
}

pub fn delete_many(_model: &ModelRef, _ids: &[&GraphqlId]) -> Vec<Delete<'static>> {
    // let mut deletes = Vec::new();

    // for chunk in ids.chunks(PARAMETER_LIMIT).into_iter() {
    //     let condition = model.fields().id().as_column().in_selection(chunk.to_vec());
    //     deletes.push(Delete::from_table(model.as_table()).so_that(condition));
    // }

    // deletes

    todo!()
}

*/
