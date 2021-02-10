use super::*;
use crate::{BsonTransform, IntoBson};
use connector_interface::Filter;
use mongodb::options::FindOptions;
use mongodb::Database;
use prisma_models::*;

pub async fn get_single_record(
    database: &Database,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
) -> crate::Result<Option<SingleRecord>> {
    let coll = database.collection(model.db_name());

    // Todo look at interfaces (req clones).
    let filter = filter.clone().into_bson()?.into_document()?;
    let find_options = FindOptions::builder()
        .projection(selected_fields.clone().into_bson()?.into_document()?)
        .build();

    let cursor = coll.find(Some(filter), Some(find_options)).await?;
    let docs = dbg!(vacuum_cursor(cursor).await?);

    if docs.len() == 0 {
        Ok(None)
    } else {
        let field_names = selected_fields
            .scalar_fields()
            .map(|sf| sf.db_name().to_owned())
            .collect::<Vec<_>>();

        let doc = docs.into_iter().next().unwrap();
        let record = document_to_record(doc, &field_names)?;

        Ok(Some(SingleRecord { record, field_names }))
    }
}
