use crate::{BsonTransform, error::MongoError, vacuum_cursor};
use itertools::Itertools;
use mongodb::{
    bson::{Bson, Document, from_bson},
    options::*,
};
use query_structure::{Model, PrismaValue};
use std::collections::HashMap;

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

#[allow(clippy::large_enum_variant)]
pub enum MongoOperation {
    Find(Option<Document>, Option<FindOptions>),
    Aggregate(Vec<Document>, Option<AggregateOptions>),
}

impl MongoCommand {
    pub fn from_raw_query(
        model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        query_type: Option<String>,
    ) -> crate::Result<MongoCommand> {
        match (query_type.as_deref(), model) {
            (Some("findRaw"), Some(m)) => Self::find(m, inputs),
            (Some("aggregateRaw"), Some(m)) => Self::aggregate(m, inputs),
            (Some("runCommandRaw"), _) => Self::raw(inputs),
            _ => unreachable!("Unexpected MongoDB raw query"),
        }
    }

    fn find(model: &Model, inputs: HashMap<String, PrismaValue>) -> crate::Result<MongoCommand> {
        let filter = inputs.get_document("filter")?;
        let options = inputs
            .get_document("options")?
            .map(Bson::Document)
            .map(from_bson::<FindOptions>)
            .transpose()?;

        Ok(Self::Handled {
            collection: model.db_name().to_owned(),
            operation: MongoOperation::Find(filter, options),
        })
    }

    fn aggregate(model: &Model, inputs: HashMap<String, PrismaValue>) -> crate::Result<MongoCommand> {
        let pipeline = inputs.get_array_document("pipeline")?.unwrap_or_default();
        let options = inputs
            .get_document("options")?
            .map(Bson::Document)
            .map(from_bson::<AggregateOptions>)
            .transpose()?;

        Ok(MongoCommand::Handled {
            collection: model.db_name().to_owned(),
            operation: MongoOperation::Aggregate(pipeline, options),
        })
    }

    fn raw(inputs: HashMap<String, PrismaValue>) -> crate::Result<MongoCommand> {
        let cmd = inputs.get_required_document("command")?;

        Ok(MongoCommand::Raw { cmd })
    }
}

trait QueryRawParsingExtension {
    fn get_document(&self, key: &str) -> crate::Result<Option<Document>>;
    fn get_required_document(&self, key: &str) -> crate::Result<Document>;
    fn get_array_document(&self, key: &str) -> crate::Result<Option<Vec<Document>>>;
}

impl QueryRawParsingExtension for HashMap<String, PrismaValue> {
    fn get_document(&self, key: &str) -> crate::Result<Option<Document>> {
        self.get(key).map(|pv| pv.try_as_bson_document(key)).transpose()
    }

    fn get_required_document(&self, key: &str) -> crate::Result<Document> {
        self.get_document(key)?
            .ok_or_else(|| MongoError::MissingRequiredArgumentError {
                argument: key.to_string(),
            })
    }

    fn get_array_document(&self, key: &str) -> crate::Result<Option<Vec<Document>>> {
        self.get(key)
            .map(|pv| {
                let stages: Vec<_> = pv
                    .try_as_bson_array(key)?
                    .into_iter()
                    .map(|stage| {
                        stage.into_document().map_err(|_| {
                            MongoError::argument_type_mismatch(key, format!("{pv:?}"), "Json::Array<Json::Object>")
                        })
                    })
                    .try_collect()?;

                Ok(stages)
            })
            .transpose()
    }
}

trait QueryRawConversionExtension {
    fn try_as_bson(&self, arg_name: &str) -> crate::Result<Bson>;
    fn try_as_bson_document(&self, arg_name: &str) -> crate::Result<Document>;
    fn try_as_bson_array(&self, arg_name: &str) -> crate::Result<Vec<Bson>>;
}

impl QueryRawConversionExtension for &PrismaValue {
    fn try_as_bson(&self, arg_name: &str) -> crate::Result<Bson> {
        match self {
            PrismaValue::Json(json) => {
                let json: serde_json::Value = serde_json::from_str(json.as_str())?;
                let bson = Bson::try_from(json)?;

                Ok(bson)
            }
            PrismaValue::List(list) => {
                let bson = list
                    .iter()
                    .map(|pv| pv.try_as_bson(arg_name))
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(Bson::Array(bson))
            }
            x => Err(MongoError::argument_type_mismatch(arg_name, format!("{x:?}"), "Json")),
        }
    }

    fn try_as_bson_document(&self, arg_name: &str) -> crate::Result<Document> {
        let bson = self.try_as_bson(arg_name)?;

        match bson {
            Bson::Document(doc) => Ok(doc),
            bson => Err(MongoError::argument_type_mismatch(
                arg_name,
                format!("{bson:?}"),
                "Json::Object",
            )),
        }
    }

    fn try_as_bson_array(&self, arg_name: &str) -> crate::Result<Vec<Bson>> {
        let bson = self.try_as_bson(arg_name)?;

        match bson {
            Bson::Array(doc) => Ok(doc),
            bson => Err(MongoError::argument_type_mismatch(
                arg_name,
                format!("{bson:?}"),
                "Json::Array",
            )),
        }
    }
}

pub async fn cursor_to_json(
    cursor: mongodb::SessionCursor<Document>,
    session: &mut mongodb::ClientSession,
) -> crate::Result<serde_json::Value> {
    let bson_result = vacuum_cursor(cursor, session)
        .await?
        .into_iter()
        .map(Bson::Document)
        .collect_vec();
    let json_result: serde_json::Value = Bson::Array(bson_result).into();

    Ok(json_result)
}
