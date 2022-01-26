use super::*;
use crate::IntoBson;
use connector_interface::{CompositeWriteOperation, ScalarWriteOperation, WriteOperation};
use itertools::Itertools;
use mongodb::bson::{doc, Document};
use prisma_models::PrismaValue;

pub trait IntoUpdateDocumentExtension {
    fn into_update_docs(self, field: &Field, field_name: &str) -> crate::Result<Vec<Document>>;
}

impl IntoUpdateDocumentExtension for WriteOperation {
    fn into_update_docs(self, field: &Field, field_name: &str) -> crate::Result<Vec<Document>> {
        match self {
            WriteOperation::Scalar(op) => op.into_update_docs(field, field_name),
            WriteOperation::Composite(op) => op.into_update_docs(field, field_name),
        }
    }
}

impl IntoUpdateDocumentExtension for ScalarWriteOperation {
    fn into_update_docs(self, field: &Field, field_name: &str) -> crate::Result<Vec<Document>> {
        let dollar_field_name = format!("${}", field_name);

        let doc = match self {
            ScalarWriteOperation::Add(rhs) if field.is_list() => match rhs {
                PrismaValue::List(vals) => {
                    let vals = vals
                        .into_iter()
                        .map(|val| (field, val).into_bson())
                        .collect::<crate::Result<Vec<_>>>()?
                        .into_iter()
                        .map(|bson| {
                            // Strip the list from the BSON values. [Todo] This is unfortunately necessary right now due to how the
                            // conversion is set up with native types, we should clean that up at some point (move from traits to fns?).
                            if let Bson::Array(mut inner) = bson {
                                inner.pop().unwrap()
                            } else {
                                bson
                            }
                        })
                        .collect();

                    let bson_array = Bson::Array(vals);

                    doc! {
                        "$set": { field_name: {
                            "$ifNull": [
                                { "$concatArrays": [dollar_field_name, bson_array.clone()] },
                                bson_array
                            ]
                        } }
                    }
                }
                val => {
                    let bson_val = match (field, val).into_bson()? {
                        bson @ Bson::Array(_) => bson,
                        bson => Bson::Array(vec![bson]),
                    };

                    doc! {
                        "$set": {
                            field_name: {
                                "$ifNull": [
                                    { "$concatArrays": [dollar_field_name, bson_val.clone()] },
                                    bson_val
                                ]
                            }
                        }
                    }
                }
            },
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            ScalarWriteOperation::Set(rhs) => doc! {
                "$set": { field_name: { "$literal": (field, rhs).into_bson()? } }
            },
            ScalarWriteOperation::Add(rhs) => doc! {
                "$set": { field_name: { "$add": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Substract(rhs) => doc! {
                "$set": { field_name: { "$subtract": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Multiply(rhs) => doc! {
                "$set": { field_name: { "$multiply": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Divide(rhs) => doc! {
                "$set": { field_name: { "$divide": [dollar_field_name, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Field(_) => unimplemented!(),
        };

        Ok(vec![doc])
    }
}

impl IntoUpdateDocumentExtension for CompositeWriteOperation {
    fn into_update_docs(self, field: &Field, field_name: &str) -> crate::Result<Vec<Document>> {
        let docs = match self {
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            CompositeWriteOperation::Set(rhs) => vec![doc! {
                "$set": { field_name: { "$literal": (field, rhs).into_bson()? } }
            }],
            CompositeWriteOperation::Update(nested_write) => {
                let docs = nested_write
                    .unfold(field)
                    .into_iter()
                    // TODO: figure out why we can't flat_map here
                    // TODO: the trait `FromIterator<Vec<mongodb::bson::Document>>` is not implemented for `std::result::Result<Vec<mongodb::bson::Document>, MongoError>`
                    .map(|(write_op, field, field_name)| write_op.into_update_docs(field, &field_name))
                    .collect::<crate::Result<Vec<_>>>()?;

                docs.into_iter().flatten().collect_vec()
            }
            CompositeWriteOperation::Unset => todo!(),
        };

        Ok(docs)
    }
}
