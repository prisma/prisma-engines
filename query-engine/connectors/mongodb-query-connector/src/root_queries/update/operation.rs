use super::{
    expression::{self, MergeDocument},
    into_expression::IntoUpdateExpression,
};

use connector_interface::FieldPath;
use mongodb::bson::{doc, Document};

#[derive(Debug, Clone)]
pub(crate) enum UpdateOperation {
    Generic(GenericOperation),
    Upsert(Upsert),
    UpdateMany(UpdateMany),
}

impl UpdateOperation {
    pub fn generic(field_path: FieldPath, expression: impl Into<expression::UpdateExpression>) -> Self {
        Self::Generic(GenericOperation {
            field_path,
            expression: expression.into(),
        })
    }

    pub fn upsert(field_path: FieldPath, set: GenericOperation, updates: Vec<UpdateOperation>) -> Self {
        Self::Upsert(Upsert {
            field_path,
            set,
            updates,
        })
    }

    pub fn update_many(field_path: FieldPath, elem_alias: String, updates: Vec<UpdateOperation>) -> Self {
        Self::UpdateMany(UpdateMany {
            elem_alias,
            field_path,
            updates,
        })
    }

    pub(crate) fn try_into_generic(self) -> Option<GenericOperation> {
        if let Self::Generic(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GenericOperation {
    pub(crate) field_path: FieldPath,
    pub(crate) expression: expression::UpdateExpression,
}

impl GenericOperation {
    /// Get a reference to the generic operation's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Upsert {
    /// The field path to which this set expression should be applied
    pub field_path: FieldPath,
    /// The set expression of the upsert
    pub set: GenericOperation,
    /// The list of updates of the upsert
    pub updates: Vec<UpdateOperation>,
}

impl Upsert {
    pub(crate) fn render_should_set_condition(field_path: &FieldPath) -> Document {
        doc! { "$eq": [{ "$ifNull": [field_path.dollar_path(true), true] }, true] }
    }

    /// Get a reference to the upsert's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpdateMany {
    /// The field path to which this set expression should be applied
    pub field_path: FieldPath,
    /// The list of updates to apply to each item of the to-many embed
    pub updates: Vec<UpdateOperation>,
    /// The alias that refers to each element of the to-many embed
    pub elem_alias: String,
}

impl UpdateMany {
    pub(crate) fn build_merge_doc(self) -> crate::Result<MergeDocument> {
        let mut merge_doc = MergeDocument::default();

        for op in self.updates {
            match op {
                UpdateOperation::Generic(generic) => {
                    let set = generic.into_update_expression()?.try_into_set().unwrap();

                    merge_doc.insert_set(set);
                }
                UpdateOperation::UpdateMany(update_many) => {
                    let set = update_many.into_update_expression()?.try_into_set().unwrap();

                    merge_doc.insert_set(set);
                }
                UpdateOperation::Upsert(upsert) => {
                    merge_doc.insert_upsert(upsert.clone(), 0)?;
                }
            }
        }

        Ok(merge_doc)
    }
}

impl From<Upsert> for UpdateOperation {
    fn from(upsert: Upsert) -> Self {
        Self::Upsert(upsert)
    }
}

impl From<GenericOperation> for UpdateOperation {
    fn from(generic: GenericOperation) -> Self {
        Self::Generic(generic)
    }
}

impl From<UpdateMany> for UpdateOperation {
    fn from(update_many: UpdateMany) -> Self {
        Self::UpdateMany(update_many)
    }
}

impl From<GenericOperation> for expression::Set {
    fn from(operation: GenericOperation) -> Self {
        Self {
            field_path: operation.field_path,
            expression: Box::new(operation.expression),
        }
    }
}
