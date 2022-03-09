use super::operation::*;
use crate::*;

use connector_interface::{CompositeWriteOperation, FieldPath, ScalarWriteOperation, WriteOperation};
use mongodb::bson::doc;
use prisma_models::{Field, PrismaValue};

pub(crate) trait IntoUpdateOperation {
    fn into_update_operations(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateOperation>>;
}

impl IntoUpdateOperation for WriteOperation {
    fn into_update_operations(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateOperation>> {
        match self {
            WriteOperation::Scalar(op) => op.into_update_operations(field, path),
            WriteOperation::Composite(op) => op.into_update_operations(field, path),
        }
    }
}

impl IntoUpdateOperation for ScalarWriteOperation {
    fn into_update_operations(self, field: &Field, field_path: FieldPath) -> crate::Result<Vec<UpdateOperation>> {
        let dollar_field_path = field_path.dollar_path(true);

        let doc = match self {
            ScalarWriteOperation::Add(rhs) if field.is_list() => render_push_update_doc(rhs, field, field_path)?,
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            ScalarWriteOperation::Set(rhs) => {
                UpdateOperation::generic(field_path, doc! { "$literal": (field, rhs).into_bson()? })
            }
            ScalarWriteOperation::Add(rhs) => UpdateOperation::generic(
                field_path,
                doc! { "$add": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Substract(rhs) => UpdateOperation::generic(
                field_path,
                doc! { "$subtract": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Multiply(rhs) => UpdateOperation::generic(
                field_path,
                doc! { "$multiply": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Divide(rhs) => UpdateOperation::generic(
                field_path,
                doc! { "$divide": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Field(_) => unimplemented!(),
        };

        Ok(vec![doc])
    }
}

impl IntoUpdateOperation for CompositeWriteOperation {
    fn into_update_operations(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateOperation>> {
        let docs = match self {
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            CompositeWriteOperation::Set(rhs) => {
                vec![UpdateOperation::generic(
                    path,
                    doc! { "$literal": (field, rhs).into_bson()? },
                )]
            }
            CompositeWriteOperation::Update(nested_write) => {
                let mut update_docs = vec![];

                for (write_op, field, field_path) in nested_write.unfold(field, path) {
                    update_docs.extend(write_op.into_update_operations(field, field_path)?);
                }

                update_docs
            }
            CompositeWriteOperation::Unset(should_unset) => {
                let mut ops = Vec::with_capacity(1);

                if should_unset {
                    ops.push(UpdateOperation::generic(path, Bson::String("$$REMOVE".to_owned())))
                }

                ops
            }
            CompositeWriteOperation::Push(rhs) => {
                vec![render_push_update_doc(rhs, field, path)?]
            }
            CompositeWriteOperation::Upsert { set, update } => {
                let set = (*set)
                    .into_update_operations(field, path.clone())?
                    .swap_remove(0)
                    .try_into_generic()
                    .unwrap();
                let updates = (*update).into_update_operations(field, path.clone())?;

                vec![UpdateOperation::upsert(path, set, updates)]
            }
            CompositeWriteOperation::UpdateMany { filter: _, update } => {
                let elem_alias = format!("{}_item", path.identifier());
                let updates = (*update).into_update_operations(field, FieldPath::new_from_alias(&elem_alias))?;

                vec![UpdateOperation::update_many(path, elem_alias, updates)]
            }
        };

        Ok(docs)
    }
}

fn render_push_update_doc(rhs: PrismaValue, field: &Field, field_path: FieldPath) -> crate::Result<UpdateOperation> {
    let dollar_field_path = field_path.dollar_path(true);

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

            UpdateOperation::generic(
                field_path,
                doc! {
                    "$ifNull": [
                        { "$concatArrays": [dollar_field_path, bson_array.clone()] },
                        bson_array
                    ]
                },
            )
        }
        val => {
            let bson_val = match (field, val).into_bson()? {
                bson @ Bson::Array(_) => bson,
                bson => Bson::Array(vec![bson]),
            };

            UpdateOperation::generic(
                field_path,
                doc! {
                    "$ifNull": [
                        { "$concatArrays": [dollar_field_path, bson_val.clone()] },
                        bson_val
                    ]
                },
            )
        }
    };

    Ok(doc)
}
