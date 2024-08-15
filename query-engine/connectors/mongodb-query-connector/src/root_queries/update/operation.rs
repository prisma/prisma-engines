use super::{expression, into_expression::IntoUpdateExpression};
use connector_interface::FieldPath;
use bson::{doc, Document};
use query_structure::Filter;

/// `UpdateOperation` is an intermediary AST used to perform preliminary transformations from a `WriteOperation`.
/// It is meant to be transformed into an `UpdateExpression`.
/// Only add new variants _if_ the operation you're adding needs to retain semantic information when transformed into an `UpdateExpression`.
/// Most of the time, it is the case when an operation has to be rendered differently when it's nested in a certain type of operation:
/// eg: an `Upsert` within an `UpdateMany` or an `Unset` within an `UpdateMany`.
/// Otherwise, use `Generic`.
#[derive(Debug, Clone)]
pub(crate) enum UpdateOperation {
    Generic(GenericOperation),
    Unset(Unset),
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

    pub fn update_many(
        field_path: FieldPath,
        filter: Filter,
        elem_alias: String,
        updates: Vec<UpdateOperation>,
    ) -> Self {
        Self::UpdateMany(UpdateMany {
            field_path,
            filter,
            elem_alias,
            updates,
        })
    }

    pub fn unset(field_path: FieldPath) -> Self {
        Self::Unset(Unset { field_path })
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
    /// The composite predicates on which updates should be applied
    pub filter: Filter,
    /// The list of updates to apply to each item of the to-many embed
    pub updates: Vec<UpdateOperation>,
    /// The alias that refers to each element of the to-many embed
    pub elem_alias: String,
}

impl UpdateMany {
    pub(crate) fn into_merge_objects_expr(self) -> crate::Result<expression::MergeObjects> {
        let mut merge_objects =
            expression::MergeObjects::new(FieldPath::new_from_alias(format!("${}", self.elem_alias)));

        for op in self.updates {
            match op {
                UpdateOperation::Generic(generic) => {
                    let set = generic.into_update_expression()?.try_into_set().unwrap();

                    merge_objects.insert_set(set);
                }
                UpdateOperation::UpdateMany(update_many) => {
                    let set = update_many.into_update_expression()?.try_into_set().unwrap();

                    merge_objects.insert_set(set);
                }
                UpdateOperation::Upsert(upsert) => {
                    merge_objects.insert_upsert(upsert)?;
                }
                UpdateOperation::Unset(unset) => {
                    merge_objects.insert_unset(unset);
                }
            }
        }

        Ok(merge_objects)
    }
}

#[derive(Debug, Clone)]
pub struct Unset {
    pub field_path: FieldPath,
}

impl Unset {
    /// Get a reference to the unset's field path.
    pub fn field_path(&self) -> &FieldPath {
        &self.field_path
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

impl From<Unset> for UpdateOperation {
    fn from(unset: Unset) -> Self {
        Self::Unset(unset)
    }
}
