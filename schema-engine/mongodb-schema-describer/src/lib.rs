//! The MongoDB Schema describer.
//!
//! A common place to query all the possible schema data we can from a MongoDB instance.

#![deny(missing_docs)]

mod schema;
mod walkers;

pub use schema::*;
pub use walkers::*;

use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, Document};

/// Describe the contents of the given database. Only bothers about the schema, meaning the
/// collection names and indexes created. Does a bit of magic to the indexes, so if having a
/// full-text index, the field info is sanitized for Prisma's use cases. We do not return `_fts` or
/// `_ftsx` fields, replacing them with the actual fields used to build the full-text index.
///
/// Be aware these text fields might not come back in the same order used when initialized.
pub async fn describe(client: &mongodb::Client, db_name: &str) -> mongodb::error::Result<MongoSchema> {
    let mut schema = MongoSchema::default();
    let database = client.database(db_name);
    let mut cursor = database.list_collections().await?;

    while let Some(collection) = cursor.try_next().await? {
        let collection_name = collection.name;
        let options = collection.options;
        let collection_type = collection.collection_type;
        let has_schema = options.validator.is_some();
        let is_capped = options.capped.is_some();

        // We need to skip views, we do not support introspecting them yet.
        if collection_type == mongodb::results::CollectionType::View {
            continue;
        }

        // We need to skip system collections, they are only used by MongoDB internally.
        // https://www.mongodb.com/docs/manual/reference/system-collections/
        if collection_type == mongodb::results::CollectionType::Collection && collection_name.starts_with("system.") {
            continue;
        }

        let collection = database.collection::<Document>(&collection_name);
        let collection_id = schema.push_collection(collection_name, has_schema, is_capped);

        let mut indexes_cursor = collection.list_indexes().await?;

        while let Some(index) = indexes_cursor.try_next().await? {
            let options = index.options.unwrap_or_default();

            let name = match options.name {
                Some(name) => name,
                None => continue,
            };

            let r#type = match (options.unique, options.text_index_version.as_ref()) {
                (Some(_), _) => IndexType::Unique,
                (_, Some(_)) => IndexType::Fulltext,
                _ => IndexType::Normal,
            };

            if name == "_id_" {
                continue; // do not introspect or diff these
            }

            if options.partial_filter_expression.is_some() {
                continue;
            }

            let as_field = |(k, v): (&String, &Bson)| {
                let property = match v.as_i32() {
                    Some(-1) => IndexFieldProperty::Descending,
                    _ => IndexFieldProperty::Ascending,
                };

                IndexField {
                    name: k.to_string(),
                    property,
                }
            };

            let fields = if r#type.is_fulltext() {
                let is_fts = |k: &str| k == "_fts" || k == "_ftsx";

                // First we take all items that are not using the special fulltext keys,
                // stopping when we find the first one.
                let head = index.keys.iter().take_while(|(k, _)| !is_fts(k)).map(as_field);

                // Then go through the weights, we have the fields presented as part of the
                // fulltext index here.
                let middle = options
                    .weights
                    .iter()
                    .flat_map(|weights| weights.keys())
                    .map(|k| IndexField {
                        name: k.to_string(),
                        property: IndexFieldProperty::Text,
                    });

                // And in the end add whatever fields were left in the index keys that are not
                // special fulltext keys.
                let tail = index
                    .keys
                    .iter()
                    .skip_while(|(k, _)| !is_fts(k))
                    .skip_while(|(k, _)| is_fts(k))
                    .map(as_field);

                head.chain(middle).chain(tail).collect()
            } else {
                index.keys.iter().map(as_field).collect()
            };

            schema.push_index(collection_id, name, r#type, fields);
        }
    }

    Ok(schema)
}
/// Get the version.
pub async fn version(client: &mongodb::Client, db_name: &str) -> mongodb::error::Result<String> {
    let database = client.database(db_name);
    use mongodb::bson::doc;
    let version_cmd = doc! {"buildInfo": 1};
    let res = database.run_command(version_cmd).await?;
    let version = res
        .get("versionArray")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_i32().unwrap().to_string())
        .collect::<Vec<String>>()
        .join(".");

    Ok(version)
}
