use super::*;
use crate::{
    filter::{convert_filter, MongoFilter},
    output_meta,
    query_builder::MongoReadQueryBuilder,
    IntoBson,
};
use connector_interface::*;
use mongodb::{
    bson::{doc, Document},
    error::ErrorKind,
    options::InsertManyOptions,
    ClientSession, Collection, Database,
};
use prisma_models::{ModelRef, PrismaValue, RecordProjection};
use std::convert::TryInto;

/// Create a single record to the database resulting in a
/// `RecordProjection` as an identifier pointing to the just-created document.
pub async fn create_record<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    mut args: WriteArgs,
) -> crate::Result<RecordProjection> {
    let coll = database.collection::<Document>(model.db_name());

    // Mongo only allows a singular ID.
    let mut id_fields = model.primary_identifier().scalar_fields().collect::<Vec<_>>();
    assert!(id_fields.len() == 1);

    let id_field = id_fields.pop().unwrap();

    // Fields to write to the document.
    // Todo: Do we need to write null for everything? There's something with nulls and exists that might impact
    //       query capability (e.g. query for field: null may need to check for exist as well?)
    let fields: Vec<_> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|field| args.has_arg_for(field.db_name()))
        .collect();

    let mut doc = Document::new();
    let id_meta = output_meta::from_field(&id_field);

    for field in fields {
        let db_name = field.db_name();
        let value = args.take_field_value(db_name).unwrap();
        let value: PrismaValue = value
            .try_into()
            .expect("Create calls can only use PrismaValue write expressions (right now).");

        let bson = (&field, value).into_bson()?;
        doc.insert(field.db_name().to_owned(), bson);
    }

    let insert_result = coll.insert_one_with_session(doc, None, session).await?;
    let id_value = value_from_bson(insert_result.inserted_id, &id_meta)?;

    Ok(RecordProjection::from((id_field, id_value)))
}

pub async fn create_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
) -> crate::Result<usize> {
    let coll = database.collection::<Document>(model.db_name());
    let num_records = args.len();
    let fields = model.fields().scalar();

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

                let bson = (field, value).into_bson()?;
                doc.insert(field_name.to_string(), bson);
            }

            Ok(doc)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    // Ordered = false (inverse of `skip_duplicates`) will ignore errors while executing
    // the operation and throw an error afterwards that we must handle.
    let options = Some(InsertManyOptions::builder().ordered(!skip_duplicates).build());

    match coll.insert_many_with_session(docs, options, session).await {
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
    args: WriteArgs,
) -> crate::Result<Vec<RecordProjection>> {
    let coll = database.collection::<Document>(model.db_name());

    // We need to load ids of documents to be updated first because Mongo doesn't
    // return ids on update, requiring us to follow the same approach as the SQL
    // connectors.
    //
    // Mongo can only have singular IDs (always `_id`), hence the unwraps. Since IDs are immutable, we also don't
    // need to merge back id changes into the result set as with SQL.
    let id_field = model.primary_identifier().scalar_fields().next().unwrap();
    let id_meta = output_meta::from_field(&id_field);

    let ids: Vec<Bson> = if let Some(selectors) = record_filter.selectors {
        selectors
            .into_iter()
            .map(|p| (&id_field, p.values().next().unwrap()).into_bson())
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = convert_filter(record_filter.filter, false, false)?;
        find_ids(coll.clone(), session, model, filter).await?
    };

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let filter = doc! { id_field.db_name(): { "$in": ids.clone() } };
    let fields = model.fields().scalar();
    let mut update_docs: Vec<Document> = vec![];

    for (field_name, write_expr) in args.args {
        let DatasourceFieldName(name) = field_name;

        // Todo: This is inefficient.
        let field = fields.iter().find(|f| f.db_name() == name).unwrap();
        let field_name = field.db_name();
        let dollar_field_name = format!("${}", field.db_name());

        let doc = match write_expr {
            WriteExpression::Add(rhs) if field.is_list => match rhs {
                PrismaValue::List(vals) => {
                    let vals = vals
                        .into_iter()
                        .map(|val| (field, val).into_bson())
                        .collect::<crate::Result<Vec<_>>>()?;
                    let bson_array = Bson::Array(vals);

                    doc! {
                        "$set": { field_name: {
                            "$ifNull": [
                                { "$concatArrays": [dollar_field_name, bson_array.clone()] },
                                bson_array
                            ]
                        } }
                    }
                }
                val => {
                    let bson_val = (field, val).into_bson()?;

                    doc! {
                        "$set": { field_name: {
                            "$ifNull": [
                                { "$concatArrays": [dollar_field_name, [bson_val.clone()]] },
                                [bson_val]
                            ]
                        } }
                    }
                }
            },
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            WriteExpression::Value(rhs) => doc! {
                "$set": { field_name: { "$literal": (field, rhs).into_bson()? } }
            },
            WriteExpression::Add(rhs) => doc! {
                "$set": { field_name: { "$add": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            WriteExpression::Substract(rhs) => doc! {
                "$set": { field_name: { "$subtract": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            WriteExpression::Multiply(rhs) => doc! {
                "$set": { field_name: { "$multiply": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            WriteExpression::Divide(rhs) => doc! {
                "$set": { field_name: { "$divide": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            WriteExpression::Field(_) => unimplemented!(),
        };

        update_docs.push(doc);
    }

    if !update_docs.is_empty() {
        coll.update_many_with_session(filter, update_docs, None, session)
            .await?;
    }

    let ids = ids
        .into_iter()
        .map(|bson_id| {
            Ok(RecordProjection::from((
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
    let id_field = model.primary_identifier().scalar_fields().next().unwrap();

    let filter = if let Some(selectors) = record_filter.selectors {
        let ids = selectors
            .into_iter()
            .map(|p| (&id_field, p.values().next().unwrap()).into_bson())
            .collect::<crate::Result<Vec<_>>>()?;

        doc! { id_field.db_name(): { "$in": ids } }
    } else {
        let filter = convert_filter(record_filter.filter, false, false)?;
        let ids = find_ids(coll.clone(), session, model, filter).await?;

        doc! { id_field.db_name(): { "$in": ids } }
    };

    let delete_result = coll.delete_many_with_session(filter, None, session).await?;

    Ok(delete_result.deleted_count as usize)
}

/// Retrives document ids based on the given filter.
async fn find_ids(
    collection: Collection<Document>,
    session: &mut ClientSession,
    model: &ModelRef,
    filter: MongoFilter,
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

    let builder = builder.with_model_projection(id_field)?;
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
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> crate::Result<()> {
    let parent_model = field.model();
    let child_model = field.related_model();

    let parent_coll = database.collection::<Document>(parent_model.db_name());
    let child_coll = database.collection::<Document>(child_model.db_name());

    let parent_id = parent_id.values().next().unwrap();
    let parent_id_field = parent_model.primary_identifier().scalar_fields().next().unwrap();
    let parent_ids_scalar_field_name = field.relation_info.fields.get(0).unwrap();
    let parent_id = (&parent_id_field, parent_id).into_bson()?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (field, value) = child_id.pairs.get(0).unwrap();
            (field, value.clone()).into_bson()
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$addToSet": { parent_ids_scalar_field_name: { "$each": child_ids.clone() } } };

    // First update the parent and add all child IDs to the m:n scalar field.
    parent_coll
        .update_one_with_session(parent_filter, parent_update, None, session)
        .await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field.related_field().relation_info.fields.get(0).unwrap().clone();
    let child_update = doc! { "$addToSet": { child_ids_scalar_field_name: parent_id } };

    child_coll
        .update_many_with_session(child_filter, child_update, None, session)
        .await?;

    Ok(())
}

pub async fn m2m_disconnect<'conn>(
    database: &Database,
    session: &mut ClientSession,
    field: &RelationFieldRef,
    parent_id: &RecordProjection,
    child_ids: &[RecordProjection],
) -> crate::Result<()> {
    let parent_model = field.model();
    let child_model = field.related_model();

    let parent_coll = database.collection::<Document>(parent_model.db_name());
    let child_coll = database.collection::<Document>(child_model.db_name());

    let parent_id = parent_id.values().next().unwrap();
    let parent_id_field = parent_model.primary_identifier().scalar_fields().next().unwrap();
    let parent_ids_scalar_field_name = field.relation_info.fields.get(0).unwrap();
    let parent_id = (&parent_id_field, parent_id).into_bson()?;

    let parent_filter = doc! { "_id": { "$eq": parent_id.clone() } };
    let child_ids = child_ids
        .iter()
        .map(|child_id| {
            let (field, value) = child_id.pairs.get(0).unwrap();
            (field, value.clone()).into_bson()
        })
        .collect::<crate::Result<Vec<_>>>()?;

    let parent_update = doc! { "$pullAll": { parent_ids_scalar_field_name: child_ids.clone() } };

    // First update the parent and remove all child IDs to the m:n scalar field.
    parent_coll
        .update_one_with_session(parent_filter, parent_update, None, session)
        .await?;

    // Then update all children and add the parent
    let child_filter = doc! { "_id": { "$in": child_ids } };
    let child_ids_scalar_field_name = field.related_field().relation_info.fields.get(0).unwrap().clone();

    let child_update = doc! { "$pull": { child_ids_scalar_field_name: parent_id } };
    child_coll
        .update_many_with_session(child_filter, child_update, None, session)
        .await?;

    Ok(())
}
