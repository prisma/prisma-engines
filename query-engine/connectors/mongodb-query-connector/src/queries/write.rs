use super::*;
use crate::IntoBson;
use connector_interface::WriteArgs;
use mongodb::{
    bson::{de::Result, Document},
    error::{BulkWriteError, Error as DriverError, ErrorKind},
    options::InsertManyOptions,
    Database,
};
use prisma_models::{ModelRef, PrismaValue, RecordProjection};
use std::{borrow::Borrow, convert::TryInto};

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
