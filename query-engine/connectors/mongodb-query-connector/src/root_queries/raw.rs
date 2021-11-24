use mongodb::{
    bson::{from_bson, Bson, Document},
    options::{
        AggregateOptions, CountOptions, DeleteOptions, DistinctOptions, FindOneAndDeleteOptions,
        FindOneAndReplaceOptions, FindOneAndUpdateOptions, FindOptions, InsertManyOptions, UpdateModifications,
        UpdateOptions,
    },
};
use std::convert::TryFrom;

use crate::{error::MongoError, BsonTransform};

impl TryFrom<Document> for MongoCommand {
    type Error = MongoError;

    fn try_from(doc: Document) -> Result<Self, Self::Error> {
        if doc.contains_key("find") {
            return MongoCommand::try_into_find(doc);
        }

        // TODO: Problem with update. We can't express all possibilities via the driver
        // if doc.contains_key("update") {
        //     let is_update_many = doc
        //         .get("multi")
        //         .map(|bson| bson.as_bool().unwrap_or(false))
        //         .unwrap_or(false);

        //     if is_update_many {
        //         return MongoCommand::try_into_update_many(doc);
        //     } else {
        //         return MongoCommand::try_into_update_one(doc);
        //     }
        // }

        if doc.contains_key("aggregate") {
            return MongoCommand::try_into_aggregate(doc);
        }

        if doc.contains_key("findAndModify") {
            return MongoCommand::try_into_find_and_modify(doc);
        }

        if doc.contains_key("count") {
            return MongoCommand::try_into_count(doc);
        }

        if doc.contains_key("distinct") {
            return MongoCommand::try_into_distinct(doc);
        }

        // TODO: later
        // if doc.contains_key("delete") {
        //     return MongoCommand::try_into_delete(doc);
        // }

        if doc.contains_key("insert") {
            return MongoCommand::try_into_insert_many(doc);
        }

        // If there is no top-level driver API for a command, just try executing it as-is with `run_command`
        Ok(MongoCommand::new_raw(doc))
    }
}

#[allow(large_enum_variant)]
pub enum MongoCommand {
    Raw {
        cmd: Document,
    },
    Handled {
        collection: String,
        operation: MongoOperation,
    },
}

pub enum MongoOperation {
    Find(Option<Document>, FindOptions),
    FindAndUpdate(Document, UpdateModifications, FindOneAndUpdateOptions),
    FindAndReplace(Document, Document, FindOneAndReplaceOptions),
    FindAndDelete(Document, FindOneAndDeleteOptions),
    Aggregate(Vec<Document>, AggregateOptions),
    Count(Option<Document>, CountOptions),
    Distinct(String, Option<Document>, DistinctOptions),
    // TODO: not implemented
    DeleteOne(Document, DeleteOptions),
    // TODO: not implemented
    DeleteMany(Document, DeleteOptions),
    // TODO: not implemented
    UpdateOne(Document, UpdateModifications, UpdateOptions),
    // TODO: not implemented
    UpdateMany(Document, UpdateModifications, UpdateOptions),
    InsertMany(Vec<Document>, InsertManyOptions),
}

impl MongoCommand {
    fn new(collection: impl Into<String>, operation: MongoOperation) -> MongoCommand {
        Self::Handled {
            collection: collection.into(),
            operation,
        }
    }

    fn new_raw(cmd: Document) -> MongoCommand {
        Self::Raw { cmd }
    }

    fn try_into_find(doc: Document) -> crate::Result<MongoCommand> {
        let options: FindOptions = from_bson(Bson::Document(doc.clone()))?;
        let collection = doc.get_required_str("find")?;
        let filter = doc.get_document_("filter")?;

        Ok(Self::new(collection, MongoOperation::Find(filter, options)))
    }

    // fn try_into_update_many(bson: Document) -> crate::Result<MongoCommand> {
    //     let doc = bson.as_document().unwrap();
    //     // TODO: convert to from_bson to MongoError
    //     let filter = doc.get("filter").map(|query| query.as_document().unwrap().clone());
    //     let options: UpdateOptions = from_bson(bson.clone())?;
    //     let update_modifications = doc
    //         .get("updates")
    //         .map(|updates| from_bson::<UpdateModifications>(updates.clone()));

    //     todo!()
    //     // Ok(Self::UpdateMany(query, options))
    // }

    // fn try_into_update_one(bson: Document) -> crate::Result<MongoCommand> {
    //     // TODO: convert to from_bson to MongoError
    //     let options: UpdateOptions = from_bson(bson.clone())?;
    //     let filter = bson
    //         .as_document()
    //         .unwrap()
    //         .get("filter")
    //         .map(|query| query.as_document().unwrap().clone());

    //     Ok(Self::Find(filter, options))
    // }

    fn try_into_aggregate(doc: Document) -> crate::Result<MongoCommand> {
        let options: AggregateOptions = from_bson(Bson::Document(doc.clone()))?;
        let collection = doc.get_required_str("aggregate")?;
        let pipeline = doc.get_required_array_document("pipeline")?;

        Ok(MongoCommand::new(
            collection,
            MongoOperation::Aggregate(pipeline, options),
        ))
    }

    fn try_into_find_and_modify(doc: Document) -> crate::Result<MongoCommand> {
        let collection = doc.get_required_str("findAndModify")?;
        let query = doc.get_document_("query")?.unwrap_or_default();

        let operation: crate::Result<MongoOperation> = match FindAndModifyType::try_from(&doc)? {
            FindAndModifyType::Update(modifications) => {
                let options: FindOneAndUpdateOptions = from_bson(Bson::Document(doc))?;

                Ok(MongoOperation::FindAndUpdate(query, modifications, options))
            }
            FindAndModifyType::Replace(replacement) => {
                let options: FindOneAndReplaceOptions = from_bson(Bson::Document(doc))?;

                Ok(MongoOperation::FindAndReplace(query, replacement, options))
            }
            FindAndModifyType::Delete => {
                let options: FindOneAndDeleteOptions = from_bson(Bson::Document(doc))?;

                Ok(MongoOperation::FindAndDelete(query, options))
            }
        };
        let operation = operation?;

        Ok(MongoCommand::new(collection, operation))
    }

    fn try_into_count(doc: Document) -> crate::Result<MongoCommand> {
        let options: CountOptions = from_bson(Bson::Document(doc.clone()))?;
        let collection = doc.get_required_str("count")?;
        let query = doc.get_document_("query")?;

        Ok(MongoCommand::new(collection, MongoOperation::Count(query, options)))
    }

    fn try_into_distinct(doc: Document) -> crate::Result<MongoCommand> {
        let options: DistinctOptions = from_bson(Bson::Document(doc.clone()))?;
        let collection = doc.get_required_str("distinct")?;
        let key = doc.get_required_str("key")?;
        let query = doc.get_document_("query")?;

        Ok(MongoCommand::new(
            collection,
            MongoOperation::Distinct(key, query, options),
        ))
    }

    // TODO: delete_one and delete_many are blocked for now
    // fn try_into_delete(bson: Document) -> crate::Result<MongoCommand> {
    //     let doc = bson.as_document().unwrap();
    //     // TODO: refactor unwrap to error
    //     let query = doc.get("query").map(|q| q.as_document().unwrap().clone());
    //     let options: DistinctOptions = from_bson(bson.clone())?;

    //     Ok(Self::DeleteOne(query, options))
    // }

    fn try_into_insert_many(doc: Document) -> crate::Result<MongoCommand> {
        let options: InsertManyOptions = from_bson(Bson::Document(doc.clone()))?;
        let collection = doc.get_required_str("insert")?;
        let documents = doc.get_required_array_document("documents")?;

        Ok(MongoCommand::new(
            collection,
            MongoOperation::InsertMany(documents, options),
        ))
    }
}

enum FindAndModifyType {
    Update(UpdateModifications),
    Replace(Document),
    Delete,
}

// TODO: Keep working on errors. Line 201 first
impl TryFrom<&Document> for FindAndModifyType {
    type Error = MongoError;

    fn try_from(doc: &Document) -> Result<Self, Self::Error> {
        if doc.contains_key("remove") {
            return Ok(Self::Delete);
        };

        match doc.get("update") {
            Some(&Bson::Array(ref stages)) => {
                let pipeline = stages
                    .iter()
                    .map(|stage| stage.clone().into_document())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(Self::Update(UpdateModifications::Pipeline(pipeline)))
            }
            Some(&Bson::Document(ref update_doc)) => {
                if update_doc.keys().any(|k| k.starts_with('$')) {
                    Ok(Self::Update(UpdateModifications::Document(update_doc.clone())))
                } else {
                    Ok(Self::Replace(update_doc.clone()))
                }
            }
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected update key to be of type document or array of document, found: {}",
                bson
            ))),
            None => Err(MongoError::QueryRawError(
                "Could not find required key update".to_string(),
            )),
        }
    }
}

trait QueryRawDocumentExtension {
    fn get_document_(&self, key: &str) -> crate::Result<Option<Document>>;
    fn get_required_document(&self, key: &str) -> crate::Result<Document>;
    fn get_required_str(&self, key: &str) -> crate::Result<String>;
    fn get_required_array_document(&self, key: &str) -> crate::Result<Vec<Document>>;
}

impl QueryRawDocumentExtension for Document {
    fn get_document_(&self, key: &str) -> crate::Result<Option<Document>> {
        match self.get(key) {
            Some(&Bson::Document(ref d)) => Ok(Some(d.clone())),
            None => Ok(None),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key {} to be of type document, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_document(&self, key: &str) -> crate::Result<Document> {
        match self.get(key) {
            Some(&Bson::Document(ref d)) => Ok(d.clone()),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key {}",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key {} to be of type document, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_str(&self, key: &str) -> crate::Result<String> {
        match self.get(key) {
            Some(&Bson::String(ref s)) => Ok(s.to_owned()),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key {}",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key {} to be of type string, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_array_document(&self, key: &str) -> crate::Result<Vec<Document>> {
        let bson_array = match self.get(key) {
            Some(&Bson::Array(ref array)) => Ok(array),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key {}",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key {} to be of type array, but found {}",
                key, bson
            ))),
        }?;

        let docs = bson_array
            .iter()
            .map(|bson_elem| bson_elem.clone().into_document())
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(docs)
    }
}
