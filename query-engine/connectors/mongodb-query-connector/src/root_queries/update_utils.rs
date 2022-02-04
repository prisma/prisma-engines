use super::*;
use crate::{BsonTransform, IntoBson};
use connector_interface::{CompositeWriteOperation, FieldPath, ScalarWriteOperation, WriteOperation};
use itertools::Itertools;
use mongodb::bson::{doc, Document};
use prisma_models::PrismaValue;

pub(crate) trait IntoUpdateDocumentExtension {
    fn into_update_docs(self) -> crate::Result<Vec<Document>>;
}

impl IntoUpdateDocumentExtension for Vec<UpdateExpression> {
    fn into_update_docs(self) -> crate::Result<Vec<Document>> {
        self.into_iter()
            .map(|op| op.into_bson()?.into_document())
            .collect::<crate::Result<Vec<_>>>()
    }
}

pub(crate) trait IntoUpdateOperationExtension {
    fn into_update_ops(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateExpression>>;
}

impl IntoUpdateOperationExtension for WriteOperation {
    fn into_update_ops(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateExpression>> {
        match self {
            WriteOperation::Scalar(op) => op.into_update_ops(field, path),
            WriteOperation::Composite(op) => op.into_update_ops(field, path),
        }
    }
}

impl IntoUpdateOperationExtension for ScalarWriteOperation {
    fn into_update_ops(self, field: &Field, field_path: FieldPath) -> crate::Result<Vec<UpdateExpression>> {
        let dollar_field_path = field_path.dollar_path();

        let doc = match self {
            ScalarWriteOperation::Add(rhs) if field.is_list() => render_push_update_doc(rhs, field, field_path)?,
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            ScalarWriteOperation::Set(rhs) => {
                UpdateExpression::set(field_path, doc! { "$literal": (field, rhs).into_bson()? })
            }
            ScalarWriteOperation::Add(rhs) => UpdateExpression::set(
                field_path,
                doc! { "$add": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Substract(rhs) => UpdateExpression::set(
                field_path,
                doc! { "$subtract": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Multiply(rhs) => UpdateExpression::set(
                field_path,
                doc! { "$multiply": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Divide(rhs) => UpdateExpression::set(
                field_path,
                doc! { "$divide": [dollar_field_path, (field, rhs).into_bson()?] },
            ),
            ScalarWriteOperation::Field(_) => unimplemented!(),
        };

        Ok(vec![doc])
    }
}

impl IntoUpdateOperationExtension for CompositeWriteOperation {
    fn into_update_ops(self, field: &Field, path: FieldPath) -> crate::Result<Vec<UpdateExpression>> {
        let dollar_field_path = path.dollar_path();

        let docs = match self {
            // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
            CompositeWriteOperation::Set(rhs) => {
                vec![UpdateExpression::set(
                    path,
                    doc! { "$literal": (field, rhs).into_bson()? },
                )]
            }
            CompositeWriteOperation::Update(nested_write) => {
                let mut update_docs = vec![];

                for (write_op, field, field_path) in nested_write.unfold(field, path) {
                    update_docs.extend(write_op.into_update_ops(field, field_path)?);
                }

                update_docs
            }
            CompositeWriteOperation::Unset(should_unset) => {
                let mut ops = Vec::with_capacity(1);

                if should_unset {
                    ops.push(UpdateExpression::set(path, Bson::String("$$REMOVE".to_owned())))
                }

                ops
            }
            CompositeWriteOperation::Push(rhs) => {
                vec![render_push_update_doc(rhs, field, path)?]
            }
            CompositeWriteOperation::Upsert { set, update } => {
                let should_set_id = format!("__prisma_should_set__{}", &path.identifier());
                let should_set_ref_id = format!("${}", &should_set_id);

                let set_docs = (*set)
                    .into_update_ops(field, path.clone())?
                    .into_iter()
                    .map(|op| {
                        let set = op.try_into_set().unwrap();
                        let cond = doc! { "$eq": [&should_set_ref_id, true] };

                        // Maps a Set expression to be executed _only_ if the field should be set and not updated. eg:
                        // From: { $set: { {field_path}: {some_expression} } }
                        // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
                        // where {cond} is the expression is the `cond` variable above
                        UpdateExpression::set_upsert(
                            set.field_path().clone(),
                            UpdateExpression::if_then_else(
                                cond,
                                set.expression().clone(),
                                Bson::String(set.field_path().dollar_path()),
                            ),
                        )
                    })
                    .collect_vec();
                let update_docs = (*update)
                    .into_update_ops(field, path)?
                    .into_iter()
                    .map(|op| match op {
                        UpdateExpression::Set(set) => {
                            // Because nested `upsert`s can be part of `update` operations,
                            // Both `CompositeWriteOperation::Upsert { set }` and `CompositeWriteOperation::Upsert { set }` operations can be found in an `upsert.update`
                            // Consequently, we need to know what type of Set expression we're dealing with to set the appropriate conditions.
                            let cond = if set.is_upsert_set {
                                doc! { "$eq": [&should_set_ref_id, true] }
                            } else {
                                doc! { "$eq": [&should_set_ref_id, false] }
                            };

                            // Maps a Set expression to be executed _only_ if the field should be set _or_ updated. eg:
                            // From: { $set: { {field_path}: {some_expression} } }
                            // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
                            // where {cond} is the expression is the `cond` variable above
                            UpdateExpression::set(
                                set.field_path().clone(),
                                UpdateExpression::if_then_else(
                                    cond,
                                    set.expression().clone(),
                                    Bson::String(set.field_path().dollar_path()),
                                ),
                            )
                        }
                        x => x,
                    })
                    .collect_vec();
                let mut docs: Vec<UpdateExpression> = vec![];

                // Adds a `__should_set__{field_path}` field that states whether a field needs to be `set` or `updated`
                // `__should_set__` is true when {field_path} is null or absent
                docs.push(
                    doc! { "$addFields": {
                        &should_set_id: {
                            "$eq": [{ "$ifNull": [dollar_field_path, true] }, true]
                        }
                    }}
                    .into(),
                );
                docs.extend(set_docs);
                docs.extend(update_docs);
                // Unsets the `__should_set__{field_path}` field
                docs.push(doc! { "$unset": Bson::from(should_set_id) }.into());

                docs
            }
        };

        Ok(docs)
    }
}

fn render_push_update_doc(rhs: PrismaValue, field: &Field, field_path: FieldPath) -> crate::Result<UpdateExpression> {
    let dollar_field_path = field_path.dollar_path();

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

            UpdateExpression::set(
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

            UpdateExpression::set(
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

#[derive(Debug, Clone)]
pub(crate) enum UpdateExpression {
    Set(Set),
    IfThenElse(IfThenElse),
    Generic(Bson),
}

impl IntoBson for UpdateExpression {
    fn into_bson(self) -> crate::Result<Bson> {
        let bson: Bson = match self {
            UpdateExpression::Set(set) => {
                doc! { "$set": { set.field_path.path(): (*set.expression).into_bson()? } }.into()
            }
            UpdateExpression::IfThenElse(if_then_else) => doc! {
                "$cond": {
                    "if": (*if_then_else.cond).into_bson()?,
                    "then": (*if_then_else.then).into_bson()?,
                    "else": (*if_then_else.els).into_bson()?
                }

            }
            .into(),
            UpdateExpression::Generic(bson) => bson,
        };

        Ok(bson)
    }
}

impl From<Bson> for UpdateExpression {
    fn from(bson: Bson) -> Self {
        Self::Generic(bson)
    }
}

impl From<Document> for UpdateExpression {
    fn from(doc: Document) -> Self {
        Self::Generic(doc.into())
    }
}

impl From<Set> for UpdateExpression {
    fn from(set: Set) -> Self {
        Self::Set(set)
    }
}

impl From<IfThenElse> for UpdateExpression {
    fn from(if_then_else: IfThenElse) -> Self {
        Self::IfThenElse(if_then_else)
    }
}

impl UpdateExpression {
    pub fn set(field_path: FieldPath, operation: impl Into<UpdateExpression>) -> Self {
        Self::Set(Set {
            field_path,
            expression: Box::new(operation.into()),
            is_upsert_set: false,
        })
    }

    /// Create a set expression specifically for an `upsert.set` operation
    pub fn set_upsert(field_path: FieldPath, operation: impl Into<UpdateExpression>) -> Self {
        Self::Set(Set {
            field_path,
            expression: Box::new(operation.into()),
            is_upsert_set: true,
        })
    }

    pub fn if_then_else(
        cond: impl Into<UpdateExpression>,
        then: impl Into<UpdateExpression>,
        els: impl Into<UpdateExpression>,
    ) -> Self {
        Self::IfThenElse(IfThenElse {
            cond: Box::new(cond.into()),
            then: Box::new(then.into()),
            els: Box::new(els.into()),
        })
    }

    fn try_into_set(self) -> Option<Set> {
        if let Self::Set(set) = self {
            Some(set)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Set {
    /// The field path to which this set expression should be applied
    pub field_path: FieldPath,
    /// The inner expression that should be set
    pub expression: Box<UpdateExpression>,
    /// Is the expression the `set` part of an `upsert`
    pub is_upsert_set: bool,
}

impl Set {
    /// Get a reference to the set's field path.
    fn field_path(&self) -> &FieldPath {
        &self.field_path
    }

    /// Get a reference to the set's expression.
    fn expression(&self) -> &UpdateExpression {
        self.expression.as_ref()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IfThenElse {
    pub cond: Box<UpdateExpression>,
    pub then: Box<UpdateExpression>,
    pub els: Box<UpdateExpression>,
}
