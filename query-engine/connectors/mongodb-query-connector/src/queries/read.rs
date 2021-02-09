use crate::IntoBsonDocument;
use connector_interface::Filter;
use futures::stream::StreamExt;
use mongodb::Database;
use mongodb::{bson::doc, options::FindOptions};
use prisma_models::*;

pub async fn get_single_record(
    database: &Database,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
) -> crate::Result<Option<SingleRecord>> {
    let coll = database.collection(model.db_name());

    let filter = filter.into_bson()?;
    let find_options = FindOptions::builder().projection(selected_fields.into_bson()?).build();

    let mut cursor = coll.find(filter, find_options).await?;
    let mut results = vec![];

    while let Some(result) = cursor.next().await {
        match result {
            Ok(document) => results.push(document),
            Err(e) => return Err(e.into()),
        }
    }

    // let query = read::get_records(&model, selected_fields.as_columns(), filter);

    // let field_names: Vec<_> = selected_fields.db_names().collect();
    // let idents = selected_fields.type_identifiers_with_arities();
    // let meta = column_metadata::create(field_names.as_slice(), idents.as_slice());

    // let record = (match conn.find(query, meta.as_slice()).await {
    //     Ok(result) => Ok(Some(result)),
    //     Err(_e @ SqlError::RecordNotFoundForWhere(_)) => Ok(None),
    //     Err(_e @ SqlError::RecordDoesNotExist) => Ok(None),
    //     Err(e) => Err(e),
    // })?
    // .map(Record::from)
    // .map(|record| SingleRecord { record, field_names });

    // Ok(record)

    todo!()
}
