use super::*;
use crate::IntoBson;
use connector_interface::{CompositeWriteOperation, ScalarWriteOperation, WriteOperation};
use mongodb::bson::{doc, Document};
use prisma_models::PrismaValue;

pub trait IntoUpdateDocumentExtension {
    fn into_update_docs(self, field: &Field, field_path: &str) -> crate::Result<Vec<Document>>;
}

impl IntoUpdateDocumentExtension for WriteOperation {
    fn into_update_docs(self, field: &Field, field_path: &str) -> crate::Result<Vec<Document>> {
        match self {
            WriteOperation::Scalar(op) => op.into_update_docs(field, field_path),
            WriteOperation::Composite(op) => op.into_update_docs(field, field_path),
        }
    }
}

impl IntoUpdateDocumentExtension for ScalarWriteOperation {
    fn into_update_docs(self, field: &Field, field_path: &str) -> crate::Result<Vec<Document>> {
        let dollar_field_path = format!("${}", field_path);

        let doc = match self {
            ScalarWriteOperation::Add(rhs) if field.is_list() => {
                render_push_update_doc(rhs, field, field_path, &dollar_field_path)?
            }
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            ScalarWriteOperation::Set(rhs) => doc! {
                "$set": { field_path: { "$literal": (field, rhs).into_bson()? } }
            },
            ScalarWriteOperation::Add(rhs) => doc! {
                "$set": { field_path: { "$add": [dollar_field_path, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Substract(rhs) => doc! {
                "$set": { field_path: { "$subtract": [dollar_field_path, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Multiply(rhs) => doc! {
                "$set": { field_path: { "$multiply": [dollar_field_path, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Divide(rhs) => doc! {
                "$set": { field_path: { "$divide": [dollar_field_path, (field, rhs).into_bson()?] } }
            },
            ScalarWriteOperation::Field(_) => unimplemented!(),
        };

        Ok(vec![doc])
    }
}

impl IntoUpdateDocumentExtension for CompositeWriteOperation {
    fn into_update_docs(self, field: &Field, field_path: &str) -> crate::Result<Vec<Document>> {
        let dollar_field_path = format!("${}", field_path);

        let docs = match self {
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            CompositeWriteOperation::Set(rhs) => vec![doc! {
                "$set": { field_path: { "$literal": (field, rhs).into_bson()? } }
            }],
            CompositeWriteOperation::Update(nested_write) => {
                let mut update_docs = vec![];

                for (write_op, field, field_path) in nested_write.unfold(field) {
                    update_docs.extend(write_op.into_update_docs(field, &field_path)?);
                }

                update_docs
            }
            CompositeWriteOperation::Unset(should_unset) => {
                let mut docs = Vec::with_capacity(1);

                if should_unset {
                    docs.push(doc! { "$unset": field_path })
                }

                docs
            }
            CompositeWriteOperation::Push(rhs) => {
                vec![render_push_update_doc(rhs, field, field_path, &dollar_field_path)?]
            }
        };

        Ok(docs)
    }
}

fn render_push_update_doc(
    rhs: PrismaValue,
    field: &Field,
    field_name: &str,
    dollar_field_name: &str,
) -> crate::Result<Document> {
    let doc = match rhs {
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
    };

    Ok(doc)
}
