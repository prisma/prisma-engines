use super::*;
use crate::error::MongoError::ConversionError;
use crate::{
    error::{DecorateErrorWithFieldInformationExtension, MongoError},
    filter::{FilterPrefix, MongoFilter, MongoFilterVisitor},
    output_meta,
    query_builder::MongoReadQueryBuilder,
    query_strings::{Aggregate, DeleteMany, DeleteOne, Find, InsertMany, InsertOne, RunCommand, UpdateMany, UpdateOne},
    root_queries::raw::{MongoCommand, MongoOperation},
    IntoBson,
};
use connector_interface::*;
use mongodb::{
    bson::{doc, Document},
    error::ErrorKind,
    options::InsertManyOptions,
    ClientSession, Collection, Database,
};
use query_structure::{Model, PrismaValue, SelectionResult};
use std::future::IntoFuture;
use std::{collections::HashMap, convert::TryInto};
use update::IntoUpdateDocumentExtension;

/// Create a single record to the database resulting in a
/// `RecordProjection` as an identifier pointing to the just-created document.
pub async fn create_record<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    mut args: WriteArgs,
) -> crate::Result<SingleRecord> {
    let coll = database.collection::<Document>(model.db_name());

    let id_field = pick_singular_id(model);

    // Fields to write to the document.
    // Todo: Do we need to write null for everything? There's something with nulls and exists that might impact
    //       query capability (e.g. query for field: null may need to check for exist as well?)
    let fields: Vec<_> = model
        .fields()
        .non_relational()
        .iter()
        .filter(|field| args.has_arg_for(field.db_name()))
        .cloned()
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

    let query_builder = InsertOne::new(&doc, coll.name());
    let insert_result = observing(&query_builder, || coll.insert_one(&doc).session(session).into_future()).await?;
    let id_value = value_from_bson(insert_result.inserted_id, &id_meta)?;

    Ok(SingleRecord {
        record: Record::new(vec![id_value]),
        field_names: vec![id_field.db_name().to_owned()],
    })
}

pub async fn create_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
) -> crate::Result<usize> {
    let coll = database.collection::<Document>(model.db_name());

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

                let bson = (field, value).into_bson().decorate_with_field_info(field)?;

                doc.insert(field_name.to_string(), bson);
            }

            Ok(doc)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    // Ordered = false (inverse of `skip_duplicates`) will ignore errors while executing
    // the operation and throw an error afterwards that we must handle.
    let ordered = !skip_duplicates;
    let options = Some(InsertManyOptions::builder().ordered(ordered).build());

    let query_string_builder = InsertMany::new(&docs, coll.name(), ordered);
    let docs_iter = docs.iter();
    let insert = observing(&query_string_builder, || {
        coll.insert_many(docs_iter)
            .with_options(options)
            .session(session)
            .into_future()
    });

    match insert.await {
        Ok(insert_result) => Ok(insert_result.inserted_ids.len()),
        Err(err) if skip_duplicates => match err.kind.as_ref() {
            ErrorKind::BulkWrite(ref failure) => {
                let errs = &failure.write_errors;
                if !errs.iter().any(|(_, err)| err.code != 11000) {
                    Ok(num_records - errs.len())
                } else {
                    Err(err.into())
                }
            }

            ErrorKind::InsertMany(ref failure) => match failure.write_errors {
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
    model: &Model,
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
            .take(match update_type {
                UpdateType::Many { limit } => limit.unwrap_or(usize::MAX),
                UpdateType::One => 1,
            } as usize)
            .map(|p| {
                (&id_field, p.values().next().unwrap())
                    .into_bson()
                    .decorate_with_scalar_field_info(&id_field)
            })
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = MongoFilterVisitor::new(FilterPrefix::default(), false).visit(record_filter.filter)?;
        find_ids(coll.clone(), session, model, filter, None).await?
    };

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let filter = doc! { id_field.db_name(): { "$in": ids.clone() } };
    let fields: Vec<_> = model
        .fields()
        .all()
        .filter_map(|field| {
            args.take_field_value(field.db_name())
                .map(|write_op| (field.clone(), write_op))
        })
        .collect();

    let mut update_docs: Vec<Document> = vec![];

    for (field, write_op) in fields {
        let field_path = FieldPath::new_from_segment(&field);

        update_docs.extend(write_op.into_update_docs(&field, field_path)?);
    }

    if !update_docs.is_empty() {
        let query_string_builder = UpdateMany::new(&filter, &update_docs, coll.name());
        let res = observing(&query_string_builder, || {
            coll.update_many(filter.clone(), update_docs.clone())
                .session(session)
                .into_future()
        })
        .await?;

        // It's important we check the `matched_count` and not the `modified_count` here.
        // MongoDB returns `modified_count: 0` when performing a noop update, which breaks
        // nested connect mutations as it rely on the returned count to know whether the update happened.
        if matches!(update_type, UpdateType::Many { limit: _ }) && res.matched_count == 0 {
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
    model: &Model,
    record_filter: RecordFilter,
    limit: Option<usize>,
) -> crate::Result<usize> {
    let coll = database.collection::<Document>(model.db_name());
    let id_field = pick_singular_id(model);

    let ids = if let Some(selectors) = record_filter.selectors {
        selectors
            .into_iter()
            .take(limit.unwrap_or(usize::MAX))
            .map(|p| {
                (&id_field, p.values().next().unwrap())
                    .into_bson()
                    .decorate_with_scalar_field_info(&id_field)
            })
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = MongoFilterVisitor::new(FilterPrefix::default(), false).visit(record_filter.filter)?;
        find_ids(coll.clone(), session, model, filter, limit).await?
    };

    if ids.is_empty() {
        return Ok(0);
    }

    let filter = doc! { id_field.db_name(): { "$in": ids } };
    let query_string_builder = DeleteMany::new(&filter, coll.name());
    let delete_result = observing(&query_string_builder, || {
        coll.delete_many(filter.clone()).session(session).into_future()
    })
    .await?;

    Ok(delete_result.deleted_count as usize)
}

pub async fn delete_record<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    record_filter: RecordFilter,
    selected_fields: FieldSelection,
) -> crate::Result<SingleRecord> {
    let coll = database.collection::<Document>(model.db_name());
    let (filter, joins) = MongoFilterVisitor::new(FilterPrefix::default(), false)
        .visit(record_filter.filter)?
        .render();
    debug_assert!(
        joins.is_empty(),
        "filter should not contain any predicates on relations"
    );

    // All filters use `aggregate` command syntax by default. To use rendered expression in `find*`
    // command family, it needs to be wrapped in `$expr`.
    let filter = doc! {
        "$expr": filter,
    };

    let query_string_builder = DeleteOne::new(&filter, coll.name());
    let document = observing(&query_string_builder, || {
        coll.find_one_and_delete(filter.clone()).session(session).into_future()
    })
    .await?
    .ok_or(MongoError::RecordDoesNotExist {
        cause: "Record to delete does not exist.".to_owned(),
    })?;

    let meta_mapping = output_meta::from_selected_fields(&selected_fields);
    let field_names: Vec<_> = selected_fields.db_names().collect();
    let record = document_to_record(document, &field_names, &meta_mapping)?;
    Ok(SingleRecord { record, field_names })
}

/// Retrives document ids based on the given filter.
async fn find_ids(
    collection: Collection<Document>,
    session: &mut ClientSession,
    model: &Model,
    filter: MongoFilter,
    limit: Option<usize>,
) -> crate::Result<Vec<Bson>> {
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

    let mut builder = builder.with_model_projection(id_field)?;

    if let Some(limit) = limit {
        builder.limit = match i64::try_from(limit) {
            Ok(limit) => Some(limit),
            Err(_) => {
                return Err(ConversionError {
                    from: "usize".to_owned(),
                    to: "i64".to_owned(),
                })
            }
        }
    }

    let query = builder.build()?;
    let docs = query.execute(collection, session).await?;
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

    let parent_ids_scalar_field_name = field.walker().fields().unwrap().next().unwrap().name().to_owned();
    let parent_id = (&parent_id_field, parent_id)
        .into_bson()
        .decorate_with_scalar_field_info(&parent_id_field)?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (selection, value) = child_id.pairs.first().unwrap();

            (selection, value.clone())
                .into_bson()
                .decorate_with_selected_field_info(selection)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$addToSet": { parent_ids_scalar_field_name: { "$each": child_ids.clone() } } };

    let query_string_builder = UpdateOne::new(&parent_filter, &parent_update, parent_coll.name());

    observing(&query_string_builder, || {
        parent_coll
            .update_one(parent_filter.clone(), parent_update.clone())
            .session(&mut *session)
            .into_future()
    })
    .await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field
        .walker()
        .opposite_relation_field()
        .unwrap()
        .fields()
        .unwrap()
        .next()
        .unwrap()
        .name()
        .to_owned();
    let child_update = doc! { "$addToSet": { child_ids_scalar_field_name: parent_id } };

    let child_updates = vec![child_update.clone()];
    let query_string_builder = UpdateMany::new(&child_filter, &child_updates, child_coll.name());
    observing(&query_string_builder, || {
        child_coll
            .update_many(child_filter.clone(), child_update.clone())
            .session(&mut *session)
            .into_future()
    })
    .await?;

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

    let parent_ids_scalar_field_name = field.walker().fields().unwrap().next().unwrap().name().to_owned();
    let parent_id = (&parent_id_field, parent_id)
        .into_bson()
        .decorate_with_scalar_field_info(&parent_id_field)?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (field, value) = child_id.pairs.first().unwrap();

            (field, value.clone())
                .into_bson()
                .decorate_with_selected_field_info(field)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$pullAll": { parent_ids_scalar_field_name: child_ids.clone() } };

    // First update the parent and remove all child IDs to the m:n scalar field.
    let query_string_builder = UpdateOne::new(&parent_filter, &parent_update, parent_coll.name());
    observing(&query_string_builder, || {
        parent_coll
            .update_one(parent_filter.clone(), parent_update.clone())
            .session(&mut *session)
            .into_future()
    })
    .await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field
        .walker()
        .opposite_relation_field()
        .unwrap()
        .fields()
        .unwrap()
        .next()
        .unwrap()
        .name()
        .to_owned();

    let child_update = doc! { "$pull": { child_ids_scalar_field_name: parent_id } };

    let child_updates = vec![child_update.clone()];
    let query_string_builder = UpdateMany::new(&child_filter, &child_updates, child_coll.name());
    observing(&query_string_builder, || {
        child_coll
            .update_many(child_filter.clone(), child_update)
            .session(session)
            .into_future()
    })
    .await?;

    Ok(())
}

/// Execute raw is not implemented on MongoDB
pub async fn execute_raw<'conn>(
    _database: &Database,
    _session: &mut ClientSession,
    _inputs: HashMap<String, PrismaValue>,
) -> crate::Result<usize> {
    Err(MongoError::Unsupported("execute_raw".into()))
}

/// Execute a plain MongoDB query, returning the answer as a JSON `Value`.
pub async fn query_raw<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: Option<&Model>,
    inputs: HashMap<String, PrismaValue>,
    query_type: Option<String>,
) -> crate::Result<RawJson> {
    let mongo_command = MongoCommand::from_raw_query(model, inputs, query_type)?;

    async {
        let json_result = match mongo_command {
            MongoCommand::Raw { cmd } => {
                let query_string_builder = RunCommand::new(&cmd);
                let mut result = observing(&query_string_builder, || {
                    database.run_command(cmd.clone()).session(session).into_future()
                })
                .await?;

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
                        let unwrapped_filter = filter.clone().unwrap_or_default();
                        let projection = options
                            .as_ref()
                            .and_then(|options| options.projection.clone())
                            .unwrap_or_default();
                        let query_string_builder = Find::new(&unwrapped_filter, &projection, coll.name());
                        let cursor = observing(&query_string_builder, || {
                            coll.find(filter.unwrap_or_default())
                                .with_options(options)
                                .session(&mut *session)
                                .into_future()
                        })
                        .await?;

                        raw::cursor_to_json(cursor, session).await?
                    }
                    MongoOperation::Aggregate(pipeline, options) => {
                        let query_string_builder = Aggregate::new(&pipeline, coll.name());
                        let cursor = observing(&query_string_builder, || {
                            coll.aggregate(pipeline.clone())
                                .with_options(options)
                                .session(&mut *session)
                                .into_future()
                        })
                        .await?;

                        raw::cursor_to_json(cursor, session).await?
                    }
                }
            }
        };

        Ok(RawJson::try_new(json_result)?)
    }
    .await
}
