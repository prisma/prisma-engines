use connector_interface::FieldPath;
use mongodb::bson::{doc, Bson, Document};

use crate::IntoBson;

#[derive(Debug, Clone)]
pub(crate) enum UpdateExpression {
    Set(Set),
    Upsert(Upsert),
    UpdateMany(UpdateMany),
    IfThenElse(IfThenElse),
    Generic(Bson),
}

impl UpdateExpression {
    pub fn set(field_path: FieldPath, operation: impl Into<UpdateExpression>) -> Self {
        Self::Set(Set {
            field_path,
            expression: Box::new(operation.into()),
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

    pub fn upsert(field_path: FieldPath, set: Set, updates: Vec<UpdateExpression>) -> Self {
        Self::Upsert(Upsert {
            field_path,
            set,
            updates,
        })
    }

    pub fn update_many(field_path: FieldPath, elem_alias: String, updates: Vec<UpdateExpression>) -> Self {
        Self::UpdateMany(UpdateMany {
            elem_alias,
            field_path,
            updates,
        })
    }

    pub fn try_into_set(self) -> Option<Set> {
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
}

impl Set {
    /// Get a reference to the set's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }

    /// Get a reference to the set's expression.
    pub(crate) fn expression(&self) -> &UpdateExpression {
        self.expression.as_ref()
    }

    pub(crate) fn into_conditional_set(self, cond: impl Into<UpdateExpression>) -> Self {
        let dollar_path = self.field_path().dollar_path(true);

        Self {
            field_path: self.field_path,
            expression: Box::new(UpdateExpression::IfThenElse(IfThenElse {
                cond: Box::new(cond.into()),
                then: self.expression,
                els: Box::new(UpdateExpression::Generic(Bson::String(dollar_path))),
            })),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IfThenElse {
    /// The condition of the if expression
    pub cond: Box<UpdateExpression>,
    /// The then branch
    pub then: Box<UpdateExpression>,
    /// The else branch
    pub els: Box<UpdateExpression>,
}

#[derive(Debug, Clone)]
pub(crate) struct Upsert {
    /// The field path to which this set expression should be applied
    pub field_path: FieldPath,
    /// The set expression of the upsert
    pub set: Set,
    /// The list of updates of the upsert
    pub updates: Vec<UpdateExpression>,
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
    pub updates: Vec<UpdateExpression>,
    /// The alias that refers to each element of the to-many embed
    pub elem_alias: String,
}

impl UpdateMany {
    /// Get a reference to the update many's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }
}

/// Represents a $mergeObjects operation.
/// Intentionally not part of the `UpdateExpression` AST for now
pub(crate) struct MergeObjects {
    pub(crate) field_path: FieldPath,
    pub(crate) expressions: Vec<UpdateExpression>,
}

impl MergeObjects {
    pub(crate) fn new(field_path: FieldPath, expressions: Vec<UpdateExpression>) -> Self {
        Self {
            field_path,
            expressions,
        }
    }

    pub(crate) fn merge_set(doc: &mut Document, set: Set) -> crate::Result<()> {
        let set_expr = set.expression.clone();
        let (path, inner_doc) = MergeObjects::fill_nested_documents(doc, set.field_path());

        inner_doc.insert(path, set_expr.into_bson()?);

        Ok(())
    }

    pub(crate) fn merge_upsert(doc: &mut Document, mut upsert: Upsert, depth: usize) -> crate::Result<()> {
        let updates_doc: crate::Result<Document> =
            upsert
                .updates
                .into_iter()
                .try_fold(Document::default(), |mut acc, u| match u {
                    UpdateExpression::Set(mut set) => {
                        set.field_path.drain(depth + 1);

                        MergeObjects::merge_set(&mut acc, set)?;

                        Ok(acc)
                    }
                    UpdateExpression::Upsert(upsert) => {
                        let mut inner_doc = Document::default();

                        MergeObjects::merge_upsert(&mut inner_doc, upsert, depth + 1)?;

                        acc.extend(inner_doc);

                        Ok(acc)
                    }
                    _ => unreachable!(),
                });

        let new_if = IfThenElse {
            cond: Box::new(
                doc! { "$eq": [Upsert::render_should_set_condition(&upsert.set.field_path().clone()), true] }.into(),
            ),
            then: upsert.set.expression.clone(),
            els: Box::new(updates_doc?.into()),
        };

        if depth > 0 {
            upsert.set.field_path.drain(depth);
        }

        doc.insert(upsert.set.field_path().path(false), new_if.into_bson()?);

        Ok(())
    }

    /// Given a mutable `Document` and a `FieldPath`, fills nested empty documents to the mutable `Document`.
    /// Returns a mutable reference to the most nested document along side the key to insert a value.
    /// ```text
    /// ["a", "b", "c"] -> ("c", doc! { "a": { "b": {} } })
    /// ```
    pub(crate) fn fill_nested_documents<'a, 'b>(
        doc: &'a mut Document,
        field_path: &'b FieldPath,
    ) -> (&'b String, &'a mut Document) {
        let (last, segments) = field_path.path.split_last().unwrap();

        let inner_doc = segments.into_iter().fold(doc, |acc, segment| {
            let inner_doc = acc
                .entry(segment.to_string())
                .or_insert(Bson::Document(Document::default()))
                .as_document_mut()
                .unwrap();

            inner_doc
        });

        (last, inner_doc)
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

impl From<Upsert> for UpdateExpression {
    fn from(upsert: Upsert) -> Self {
        Self::Upsert(upsert)
    }
}
