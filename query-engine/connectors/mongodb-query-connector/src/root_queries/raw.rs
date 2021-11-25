use mongodb::{
    bson::{from_bson, Bson, Document},
    options::*,
};
use std::convert::TryFrom;

use crate::{error::MongoError, BsonTransform};

impl TryFrom<Document> for MongoCommand {
    type Error = MongoError;

    fn try_from(doc: Document) -> Result<Self, Self::Error> {
        if doc.contains_key("find") {
            return MongoCommand::try_into_find(doc);
        }

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

        if doc.contains_key("insert") {
            return MongoCommand::try_into_insert_many(doc);
        }

        // If there is no top-level driver API for a command or the driver doesn't allow expressing everything that can be done via the raw command
        // just try executing it as-is with `run_command`
        Ok(MongoCommand::new_raw(doc))
    }
}

#[allow(clippy::large_enum_variant)]
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
    // Collection commands
    Find(Option<Document>, FindOptions),
    FindAndUpdate(Document, UpdateModifications, FindOneAndUpdateOptions),
    FindAndReplace(Document, Document, FindOneAndReplaceOptions),
    FindAndDelete(Document, FindOneAndDeleteOptions),
    Aggregate(Vec<Document>, AggregateOptions),
    Count(Option<Document>, CountOptions),
    Distinct(String, Option<Document>, DistinctOptions),
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
        // Mongo driver options name this field "return_document".
        let new = doc.get_bool("new").unwrap_or(false);
        // Mongo driver options name this field "projection".
        let projection = doc.get_document_("fields")?;

        let operation: crate::Result<MongoOperation> = match FindAndModifyType::try_from(&doc)? {
            FindAndModifyType::Update(modifications) => {
                let mut options: FindOneAndUpdateOptions = from_bson(Bson::Document(doc))?;

                if new {
                    options.return_document = Some(ReturnDocument::After);
                }

                options.projection = projection;

                Ok(MongoOperation::FindAndUpdate(query, modifications, options))
            }
            FindAndModifyType::Replace(replacement) => {
                let mut options: FindOneAndReplaceOptions = from_bson(Bson::Document(doc))?;

                if new {
                    options.return_document = Some(ReturnDocument::After);
                }

                options.projection = projection;

                Ok(MongoOperation::FindAndReplace(query, replacement, options))
            }
            FindAndModifyType::Delete => {
                let mut options: FindOneAndDeleteOptions = from_bson(Bson::Document(doc))?;

                options.projection = projection;

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
                "Expected 'update' key to be of type document or array of document, found: {}",
                bson
            ))),
            None => Err(MongoError::QueryRawError(
                "Either an 'update' or 'remove' key must be specified".to_string(),
            )),
        }
    }
}

pub(crate) trait QueryRawDocumentExtension {
    fn get_document_(&self, key: &str) -> crate::Result<Option<Document>>;
    fn get_required_document(&self, key: &str) -> crate::Result<Document>;
    fn get_required_str(&self, key: &str) -> crate::Result<String>;
    fn get_required_array_document(&self, key: &str) -> crate::Result<Vec<Document>>;
    /// Removes unnecessary properties from raw response
    /// See https://docs.mongodb.com/v5.0/reference/method/db.runCommand
    fn cleanup_raw_result(&mut self);
}

impl QueryRawDocumentExtension for Document {
    fn get_document_(&self, key: &str) -> crate::Result<Option<Document>> {
        match self.get(key) {
            Some(&Bson::Document(ref d)) => Ok(Some(d.clone())),
            None => Ok(None),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key '{}' to be of type document, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_document(&self, key: &str) -> crate::Result<Document> {
        match self.get(key) {
            Some(&Bson::Document(ref d)) => Ok(d.clone()),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key '{}'",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key '{}' to be of type document, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_str(&self, key: &str) -> crate::Result<String> {
        match self.get(key) {
            Some(&Bson::String(ref s)) => Ok(s.to_owned()),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key '{}'",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key '{}' to be of type string, but found {}",
                key, bson
            ))),
        }
    }

    fn get_required_array_document(&self, key: &str) -> crate::Result<Vec<Document>> {
        let bson_array = match self.get(key) {
            Some(&Bson::Array(ref array)) => Ok(array),
            None => Err(MongoError::QueryRawError(format!(
                "Could not find required key '{}'",
                key
            ))),
            Some(bson) => Err(MongoError::QueryRawError(format!(
                "Expected key '{}' to be of type array, but found {}",
                key, bson
            ))),
        }?;

        let docs = bson_array
            .iter()
            .map(|bson_elem| bson_elem.clone().into_document())
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(docs)
    }

    fn cleanup_raw_result(&mut self) {
        self.remove("operationTime");
        self.remove("$clusterTime");
        self.remove("opTime");
        self.remove("electionId");
    }
}
