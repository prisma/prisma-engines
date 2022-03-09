use crate::*;
use mongodb::bson::{doc, Bson, Document};

pub(crate) fn flatten_bson(bson: Bson) -> crate::Result<Vec<Document>> {
    let mut update_docs = vec![];

    match bson {
        Bson::Array(bson_arr) => {
            for bson_elem in bson_arr {
                update_docs.extend(flatten_bson(bson_elem)?);
            }
        }
        bson => {
            update_docs.push(bson.into_document()?);
        }
    }

    Ok(update_docs)
}