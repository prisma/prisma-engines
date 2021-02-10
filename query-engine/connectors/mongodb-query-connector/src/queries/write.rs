use super::*;
use crate::IntoBson;
use connector_interface::WriteArgs;
use mongodb::{bson::Document, Database};
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
