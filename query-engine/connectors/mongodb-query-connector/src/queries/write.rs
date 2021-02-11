use super::*;
use crate::{BsonTransform, IntoBson};
use connector_interface::*;
use mongodb::{
    bson::{doc, Document},
    error::ErrorKind,
    options::{FindOptions, InsertManyOptions},
    Database,
};
use prisma_models::{ModelRef, PrismaValue, RecordProjection};
use std::convert::TryInto;

/// Create a single record to the database resulting in a
/// `RecordProjection` as an identifier pointing to the just-created document.
pub async fn create_record(
    database: &Database,
    model: &ModelRef,
    mut args: WriteArgs,
) -> crate::Result<RecordProjection> {
    let coll = database.collection(model.db_name());

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
        .filter(|field| args.has_arg_for(&field.db_name()))
        .collect();

    let mut doc = Document::new();

    for field in fields {
        let db_name = field.db_name();
        let value = args.take_field_value(db_name).unwrap();
        let value: PrismaValue = value
            .try_into()
            .expect("Create calls can only use PrismaValue write expressions (right now).");

        let bson = value.into_bson()?;
        doc.insert(field.db_name().to_owned(), bson);
    }

    let insert_result = coll.insert_one(doc, None).await?;
    let id_value = value_from_bson(insert_result.inserted_id)?;

    Ok(RecordProjection::from((id_field, id_value)))
}

pub async fn create_records(
    database: &Database,
    model: &ModelRef,
    args: Vec<WriteArgs>,
    skip_duplicates: bool,
) -> crate::Result<usize> {
    let coll = database.collection(model.db_name());
    let num_records = args.len();

    let docs = args
        .into_iter()
        .map(|arg| {
            let mut doc = Document::new();

            for (field, value) in arg.args {
                let value: PrismaValue = value
                    .try_into()
                    .expect("Create calls can only use PrismaValue write expressions (right now).");

                let bson = value.into_bson()?;
                doc.insert(field.to_string(), bson);
            }

            Ok(doc)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    // Ordered = false (inverse of `skip_duplicates`) will ignore errors while executing
    // the operation and throw an error afterwards that we must handle.
    let options = Some(InsertManyOptions::builder().ordered(Some(!skip_duplicates)).build());

    match coll.insert_many(docs, options).await {
        Ok(insert_result) => Ok(insert_result.inserted_ids.len()),
        Err(err) if skip_duplicates => match err.kind.as_ref() {
            ErrorKind::BulkWriteError(ref failure) => match failure.write_errors {
                Some(ref errs) if !errs.iter().any(|err| err.code != 11000) => Ok(num_records - errs.len()),
                _ => Err(err.into()),
            },

            _ => Err(err.into()),
        },

        Err(err) => Err(err.into()),
    }
}

pub async fn update_records(
    database: &Database,
    model: &ModelRef,
    record_filter: RecordFilter,
    args: WriteArgs,
) -> crate::Result<Vec<RecordProjection>> {
    let coll = database.collection(model.db_name());

    // We need to load ids of documents to be updated first because Mongo doesn't
    // return ids on update, requiring us to follow the same approach as the SQL
    // connectors.
    //
    // Mongo can only have singular IDs (always `_id`), hence the unwraps. Since IDs are immutable, we also don't
    // need to merge back id changes into the result set as with SQL.
    let id_field = model.primary_identifier().scalar_fields().next().unwrap();
    let ids: Vec<Bson> = if let Some(selectors) = record_filter.selectors {
        selectors
            .into_iter()
            .map(|p| (&id_field, p.values().next().unwrap()).into_bson())
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        let filter = record_filter.filter.into_bson()?.into_document()?;
        let find_options = FindOptions::builder()
            .projection(doc! { id_field.db_name(): 1 })
            .build();

        let cursor = coll.find(Some(filter), Some(find_options)).await?;
        let docs = vacuum_cursor(cursor).await?;

        docs.into_iter()
            .map(|mut doc| doc.remove(id_field.db_name()).unwrap())
            .collect()
    };

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let filter = doc! { id_field.db_name(): { "$in": ids.clone() } };
    let mut update_doc = Document::new();

    for (field_name, write_expr) in args.args {
        let DatasourceFieldName(name) = field_name;

        let (op_key, val) = match write_expr {
            WriteExpression::Field(_) => unimplemented!(),
            WriteExpression::Value(rhs) => ("$set", rhs.into_bson()?),
            WriteExpression::Add(rhs) => ("$inc", rhs.into_bson()?),
            WriteExpression::Substract(rhs) => ("$inc", (rhs * PrismaValue::Int(-1)).into_bson()?),
            WriteExpression::Multiply(rhs) => ("$mul", rhs.into_bson()?),
            WriteExpression::Divide(rhs) => ("$mul", (PrismaValue::new_float(1.0) / rhs).into_bson()?),
        };

        let entry = update_doc.entry(op_key.to_owned()).or_insert(Document::new().into());
        entry.as_document_mut().unwrap().insert(name, val);
    }

    let _update_result = coll.update_many(filter, update_doc, None).await?;
    let ids = ids
        .into_iter()
        .map(|bson_id| Ok(RecordProjection::from((id_field.clone(), value_from_bson(bson_id)?))))
        .collect::<crate::Result<Vec<_>>>()?;

    Ok(ids)
}
