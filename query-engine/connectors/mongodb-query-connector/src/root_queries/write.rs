use super::*;
use crate::{
    error::DecorateErrorWithFieldInformationExtension,
    filter::{FilterPrefix, MongoFilter, MongoFilterVisitor},
    logger, output_meta,
    query_builder::MongoReadQueryBuilder,
    root_queries::raw::{MongoCommand, MongoOperation},
    BsonTransform, IntoBson,
};
use connector_interface::*;
use mongodb::{
    bson::{doc, Document},
    error::ErrorKind,
    options::InsertManyOptions,
    ClientSession, Collection, Database,
};
use prisma_models::{ModelRef, PrismaValue, SelectionResult};
use std::{collections::HashMap, convert::TryInto};
use tracing::{info_span, Instrument};
use update::IntoUpdateDocumentExtension;

/// Create a single record to the database resulting in a
/// `RecordProjection` as an identifier pointing to the just-created document.
pub async fn create_record<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    mut args: WriteArgs,
) -> crate::Result<SelectionResult> {
    let coll = database.collection::<Document>(model.db_name());

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &format_args!("db.{}.insertOne(*)", coll.name())
    );

    let id_field = pick_singular_id(model);

    // Fields to write to the document.
    // Todo: Do we need to write null for everything? There's something with nulls and exists that might impact
    //       query capability (e.g. query for field: null may need to check for exist as well?)
    let fields: Vec<_> = model
        .fields()
        .non_relational()
        .iter()
        .filter(|field| args.has_arg_for(field.db_name()))
        .map(Clone::clone)
        .collect();

    let mut doc = Document::new();
    let id_meta = output_meta::from_scalar_field(&id_field);

    for field in fields {
        let db_name = field.db_name();
        let value = args.take_field_value(db_name).unwrap();
        let value: PrismaValue = value
            .try_into()
            .expect("Create calls can only use PrismaValue write expressions (right now).");

        let bson = (&field, value).into_bson().decorate_with_field_info(&field)?;

        doc.insert(field.db_name().to_owned(), bson);
    }

    logger::log_insert_one(coll.name(), &doc);

    let insert_result = metrics(|| coll.insert_one_with_session(doc, None, session))
        .instrument(span)
        .await?;
    let id_value = value_from_bson(insert_result.inserted_id, &id_meta)?;

    Ok(SelectionResult::from((id_field, id_value)))
}

pub async fn create_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
) -> crate::Result<usize> {
    let coll = database.collection::<Document>(model.db_name());

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &format_args!("db.{}.insertMany(*)", coll.name())
    );

    let num_records = args.len();
    let fields: Vec<_> = model.fields().non_relational();

    let docs = args
        .into_iter()
        .map(|arg| {
            let mut doc = Document::new();

            for (field_name, value) in arg.args {
                // Todo: This is inefficient.
                let field = fields.iter().find(|f| f.db_name() == &*field_name).unwrap();
                let value: PrismaValue = value
                    .try_into()
                    .expect("Create calls can only use PrismaValue write expressions (right now).");

                let bson = (field, value).into_bson().decorate_with_field_info(&field)?;

                doc.insert(field_name.to_string(), bson);
            }

            Ok(doc)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    // Ordered = false (inverse of `skip_duplicates`) will ignore errors while executing
    // the operation and throw an error afterwards that we must handle.
    let ordered = !skip_duplicates;
    let options = Some(InsertManyOptions::builder().ordered(ordered).build());

    logger::log_insert_many(coll.name(), &docs, ordered);

    let insert = metrics(|| coll.insert_many_with_session(docs, options, session)).instrument(span);

    match insert.await {
        Ok(insert_result) => Ok(insert_result.inserted_ids.len()),
        Err(err) if skip_duplicates => match err.kind.as_ref() {
            ErrorKind::BulkWrite(ref failure) => match failure.write_errors {
                Some(ref errs) if !errs.iter().any(|err| err.code != 11000) => Ok(num_records - errs.len()),
                _ => Err(err.into()),
            },

            _ => Err(err.into()),
        },

        Err(err) => Err(err.into()),
    }
}

pub async fn update_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    record_filter: RecordFilter,
    mut args: WriteArgs,
    update_type: UpdateType,
) -> crate::Result<Vec<SelectionResult>> {
    let coll = database.collection::<Document>(model.db_name());

    // We need to load ids of documents to be updated first because Mongo doesn't
    // return ids on update, requiring us to follow the same approach as the SQL
    // connectors.
    //
    // Mongo can only have singular IDs (always `_id`), hence the unwraps. Since IDs are immutable, we also don't
    // need to merge back id changes into the result set as with SQL.
    let id_field = pick_singular_id(model);
    let id_meta = output_meta::from_scalar_field(&id_field);
    let ids: Vec<Bson> = if let Some(selectors) = record_filter.selectors {
        selectors
            .into_iter()
            .map(|p| {
                (&id_field, p.values().next().unwrap())
                    .into_bson()
                    .decorate_with_scalar_field_info(&id_field)
            })
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = MongoFilterVisitor::new(FilterPrefix::default(), false).visit(record_filter.filter)?;
        find_ids(database, coll.clone(), session, model, filter).await?
    };

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &format_args!("db.{}.updateMany(*)", coll.name())
    );

    let filter = doc! { id_field.db_name(): { "$in": ids.clone() } };
    let fields: Vec<_> = model
        .fields()
        .all
        .iter()
        .filter_map(|field| {
            args.take_field_value(field.db_name())
                .map(|write_op| (field.clone(), write_op))
        })
        .collect();

    let mut update_docs: Vec<Document> = vec![];

    //merging of set and unset operations should happen here
    //to do that write_op.into_update_docs should not already return a doc but a vec<>
    use crate::root_queries::update::expression::MergedSet;
    use crate::root_queries::update::into_expression::IntoUpdateExpressions;
    use crate::root_queries::update::into_operation::IntoUpdateOperation;
    use crate::root_queries::update::operation::UpdateOperation;
    let mut set = MergedSet { pairs: vec![] };
    for (field, write_op) in fields {
        let field_path = FieldPath::new_from_segment(&field);
        let operations = write_op.into_update_operations(&field, field_path)?;
        let mut expressions = vec![];

        for op in operations {
            match op {
                UpdateOperation::Generic(set_op) => set.pairs.push((set_op.field_path, Box::new(set_op.expression))),
                _ => expressions.extend(op.into_update_expressions()?),
            }
        }

        for expr in expressions {
            update_docs.push(expr.into_bson()?.into_document()?);
        }
    }

    if !set.pairs.is_empty() {
        update_docs.push(set.into_bson()?.into_document()?);
    }

    // println!("{:?}", update_docs);

    if !update_docs.is_empty() {
        logger::log_update_many_vec(coll.name(), &filter, &update_docs);
        let res = metrics(|| coll.update_many_with_session(filter, update_docs, None, session))
            .instrument(span)
            .await?;

        if update_type == UpdateType::Many && res.modified_count == 0 {
            return Ok(Vec::new());
        }
    }

    let ids = ids
        .into_iter()
        .map(|bson_id| {
            Ok(SelectionResult::from((
                id_field.clone(),
                value_from_bson(bson_id, &id_meta)?,
            )))
        })
        .collect::<crate::Result<Vec<_>>>()?;

    Ok(ids)
}

pub async fn delete_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    record_filter: RecordFilter,
) -> crate::Result<usize> {
    let coll = database.collection::<Document>(model.db_name());
    let id_field = pick_singular_id(model);

    let ids = if let Some(selectors) = record_filter.selectors {
        selectors
            .into_iter()
            .map(|p| {
                (&id_field, p.values().next().unwrap())
                    .into_bson()
                    .decorate_with_scalar_field_info(&id_field)
            })
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = MongoFilterVisitor::new(FilterPrefix::default(), false).visit(record_filter.filter)?;
        find_ids(database, coll.clone(), session, model, filter).await?
    };

    if ids.is_empty() {
        return Ok(0);
    }

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &format_args!("db.{}.deleteMany(*)", coll.name())
    );

    let filter = doc! { id_field.db_name(): { "$in": ids } };
    logger::log_delete_many(coll.name(), &filter);
    let delete_result = metrics(|| coll.delete_many_with_session(filter, None, session))
        .instrument(span)
        .await?;

    Ok(delete_result.deleted_count as usize)
}

/// Retrives document ids based on the given filter.
async fn find_ids(
    database: &Database,
    collection: Collection<Document>,
    session: &mut ClientSession,
    model: &ModelRef,
    filter: MongoFilter,
) -> crate::Result<Vec<Bson>> {
    let coll = database.collection::<Document>(model.db_name());

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &format_args!("db.{}.findMany(*)", coll.name())
    );

    let id_field = model.primary_identifier();
    let mut builder = MongoReadQueryBuilder::new(model.clone());

    // If a filter comes with joins, it needs to be run _after_ the initial filter query / $matches.
    let (filter, filter_joins) = filter.render();
    if !filter_joins.is_empty() {
        builder.joins = filter_joins;
        builder.join_filters.push(filter);
    } else {
        builder.query = Some(filter);
    };

    let builder = builder.with_model_projection(id_field)?;
    let query = builder.build()?;
    let docs = query.execute(collection, session).instrument(span).await?;
    let ids = docs.into_iter().map(|mut doc| doc.remove("_id").unwrap()).collect();

    Ok(ids)
}

/// Connect relations defined in `child_ids` to a parent defined in `parent_id`.
/// The relation information is in the `RelationFieldRef`.
pub async fn m2m_connect<'conn>(
    database: &Database,
    session: &mut ClientSession,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
) -> crate::Result<()> {
    let parent_model = field.model();
    let child_model = field.related_model();

    let parent_coll = database.collection::<Document>(parent_model.db_name());
    let child_coll = database.collection::<Document>(child_model.db_name());

    let parent_id = parent_id.values().next().unwrap();
    let parent_id_field = pick_singular_id(&parent_model);

    let parent_ids_scalar_field_name = field.relation_info.fields.get(0).unwrap();
    let parent_id = (&parent_id_field, parent_id)
        .into_bson()
        .decorate_with_scalar_field_info(&parent_id_field)?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (selection, value) = child_id.pairs.get(0).unwrap();

            (selection, value.clone())
                .into_bson()
                .decorate_with_selected_field_info(&selection)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$addToSet": { parent_ids_scalar_field_name: { "$each": child_ids.clone() } } };

    logger::log_update_one(parent_coll.name(), &parent_filter, &parent_update);
    // First update the parent and add all child IDs to the m:n scalar field.
    metrics(|| parent_coll.update_one_with_session(parent_filter, parent_update, None, session)).await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field.related_field().relation_info.fields.get(0).unwrap().clone();
    let child_update = doc! { "$addToSet": { child_ids_scalar_field_name: parent_id } };

    // this needs work
    logger::log_update_many(child_coll.name(), &child_filter, &child_update);

    metrics(|| child_coll.update_many_with_session(child_filter, child_update, None, session)).await?;

    Ok(())
}

pub async fn m2m_disconnect<'conn>(
    database: &Database,
    session: &mut ClientSession,
    field: &RelationFieldRef,
    parent_id: &SelectionResult,
    child_ids: &[SelectionResult],
) -> crate::Result<()> {
    let parent_model = field.model();
    let child_model = field.related_model();

    let parent_coll = database.collection::<Document>(parent_model.db_name());
    let child_coll = database.collection::<Document>(child_model.db_name());

    let parent_id = parent_id.values().next().unwrap();
    let parent_id_field = pick_singular_id(&parent_model);

    let parent_ids_scalar_field_name = field.relation_info.fields.get(0).unwrap();
    let parent_id = (&parent_id_field, parent_id)
        .into_bson()
        .decorate_with_scalar_field_info(&parent_id_field)?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (field, value) = child_id.pairs.get(0).unwrap();

            (field, value.clone())
                .into_bson()
                .decorate_with_selected_field_info(&field)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$pullAll": { parent_ids_scalar_field_name: child_ids.clone() } };

    // First update the parent and remove all child IDs to the m:n scalar field.
    logger::log_update_one(parent_coll.name(), &parent_filter, &parent_update);
    metrics(|| parent_coll.update_one_with_session(parent_filter, parent_update, None, session)).await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field.related_field().relation_info.fields.get(0).unwrap().clone();

    let child_update = doc! { "$pull": { child_ids_scalar_field_name: parent_id } };
    logger::log_update_many(child_coll.name(), &child_filter, &child_update);

    metrics(|| child_coll.update_many_with_session(child_filter, child_update, None, session)).await?;

    Ok(())
}

/// Execute raw is not implemented on MongoDB
pub async fn execute_raw<'conn>(
    _database: &Database,
    _session: &mut ClientSession,
    _inputs: HashMap<String, PrismaValue>,
) -> crate::Result<usize> {
    unimplemented!()
}

/// Execute a plain MongoDB query, returning the answer as a JSON `Value`.
pub async fn query_raw<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: Option<&ModelRef>,
    inputs: HashMap<String, PrismaValue>,
    query_type: Option<String>,
) -> crate::Result<serde_json::Value> {
    let db_statement = get_raw_db_statement(&query_type, &model, database);
    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.statement" = &&db_statement.as_str()
    );

    let mongo_command = MongoCommand::from_raw_query(model, inputs, query_type)?;

    async {
        let json_result = match mongo_command {
            MongoCommand::Raw { cmd } => {
                let mut result = metrics(|| database.run_command_with_session(cmd, None, session)).await?;

                // Removes unnecessary properties from raw response
                // See https://docs.mongodb.com/v5.0/reference/method/db.runCommand
                result.remove("operationTime");
                result.remove("$clusterTime");
                result.remove("opTime");
                result.remove("electionId");

                let json_result: serde_json::Value = Bson::Document(result).into();

                json_result
            }
            MongoCommand::Handled { collection, operation } => {
                let coll = database.collection::<Document>(collection.as_str());

                match operation {
                    MongoOperation::Find(filter, options) => {
                        let cursor = coll.find_with_session(filter, options, session).await?;

                        raw::cursor_to_json(cursor, session).await?
                    }
                    MongoOperation::Aggregate(pipeline, options) => {
                        let cursor = coll.aggregate_with_session(pipeline, options, session).await?;

                        raw::cursor_to_json(cursor, session).await?
                    }
                }
            }
        };
        Ok(json_result)
    }
    .instrument(span)
    .await
}

fn get_raw_db_statement(query_type: &Option<String>, model: &Option<&ModelRef>, database: &Database) -> String {
    match (query_type.as_deref(), model) {
        (Some("findRaw"), Some(m)) => format!("db.{}.findRaw(*)", database.collection::<Document>(m.db_name()).name()),
        (Some("aggregateRaw"), Some(m)) => format!(
            "db.{}.aggregateRaw(*)",
            database.collection::<Document>(m.db_name()).name()
        ),
        _ => "db.runCommandRaw(*)".to_string(),
    }
}
