use super::{into_expression::IntoUpdateExpression, operation};

use connector_interface::FieldPath;
use indexmap::IndexMap;
use mongodb::bson::{doc, Bson, Document};

/// `UpdateExpression` is an intermediary AST that's used to represent MongoDB expressions.
/// It is meant to be transformed into `BSON`.
/// Only add new variants _if_ the expression needs to retain semantic information when transformed into `BSON` or another `UpdateExpression`.
/// Otherwise, use `Generic`.
#[derive(Debug, Clone)]
pub(crate) enum UpdateExpression {
    /// A `$set` expression
    Set(Set),
    /// A `$cond` expression
    IfThenElse(IfThenElse),
    /// A `$mergeObjects` expression
    MergeObjects(MergeObjects),
    /// A document `{ ... }` expression
    /// (used to retain semantic information on the values (as opposed to a generic Bson::Document))
    MergeDocument(MergeDocument),
    /// Anything else
    Generic(Bson),
}

impl UpdateExpression {
    pub fn set(field_path: FieldPath, expression: impl Into<UpdateExpression>) -> Self {
        Self::Set(Set {
            field_path,
            expression: Box::new(expression.into()),
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

    pub(crate) fn try_into_set(self) -> Option<Set> {
        if let Self::Set(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(crate) fn as_merge_objects_mut(&mut self) -> Option<&mut MergeObjects> {
        if let Self::MergeObjects(ref mut v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Set {
    /// The field path to which this $set expression should be applied
    pub field_path: FieldPath,
    /// The inner expression that should be set
    pub expression: Box<UpdateExpression>,
}

impl Set {
    /// Get a reference to the set's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }

    /// Transforms a `Set` expression into a conditional one. eg:
    /// ```text
    /// from: { $set: { {set.field_path}: {set.expression} } }
    /// to:   { $set: { $cond: { if: {cond}, then: {set.expression}, else: "${set.field_path}"  } } }
    /// ```
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

#[derive(Debug, Clone, Default)]
pub(crate) struct MergeDocument {
    pub inner: IndexMap<String, UpdateExpression>,
}

impl IntoIterator for MergeDocument {
    type Item = (String, UpdateExpression);
    type IntoIter = indexmap::map::IntoIter<String, UpdateExpression>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl MergeDocument {
    /// Get a mutable reference to the merge document's inner.
    pub(crate) fn inner_mut(&mut self) -> &mut IndexMap<String, UpdateExpression> {
        &mut self.inner
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MergeObjects {
    /// The field path to which this $mergeObjects expression should be applied
    pub(crate) field_path: FieldPath,
    /// Hack: keys of the document located in `field_path` to unset
    pub(crate) keys_to_unset: Vec<String>,
    /// Document against which it should be merged
    pub(crate) document: MergeDocument,
}

impl MergeObjects {
    pub(crate) fn new(field_path: FieldPath) -> Self {
        Self {
            field_path,
            document: MergeDocument::default(),
            keys_to_unset: vec![],
        }
    }

    /// Get a reference to the merge objects's field path.
    pub(crate) fn field_path(&self) -> &FieldPath {
        &self.field_path
    }

    /// Get a mutable reference to the merge objects's doc.
    pub(crate) fn doc_mut(&mut self) -> &mut IndexMap<String, UpdateExpression> {
        self.document.inner_mut()
    }

    pub(crate) fn insert_set(&mut self, set: Set) {
        let set_expr = set.expression.clone();
        let (path, inner_merge) = self.fill_empty_documents(set.field_path());

        inner_merge.doc_mut().insert(path.to_owned(), *set_expr);
    }

    pub(crate) fn insert_upsert(&mut self, mut upsert: operation::Upsert, depth: usize) -> crate::Result<()> {
        let mut merge_doc = MergeObjects::new(upsert.field_path);

        for op in upsert.updates {
            match op {
                operation::UpdateOperation::Generic(generic) => {
                    let mut set = generic.into_update_expression()?.try_into_set().unwrap();
                    set.field_path.drain(depth + 1);

                    merge_doc.insert_set(set);
                }
                operation::UpdateOperation::UpdateMany(update_many) => {
                    let mut set = update_many.into_update_expression()?.try_into_set().unwrap();
                    set.field_path.drain(depth + 1);

                    merge_doc.insert_set(set);
                }
                operation::UpdateOperation::Upsert(upsert) => {
                    let mut inner_merge = MergeObjects::new(upsert.field_path.clone());
                    inner_merge.insert_upsert(upsert, depth + 1)?;

                    merge_doc.doc_mut().extend(inner_merge.document);
                }
                operation::UpdateOperation::Unset(unset) => {
                    merge_doc.insert_unset(unset);
                }
            }
        }

        let new_if = UpdateExpression::if_then_else(
            doc! { "$eq": [operation::Upsert::render_should_set_condition(&upsert.set.field_path().clone()), true] },
            upsert.set.expression.clone(),
            merge_doc,
        );

        if depth > 0 {
            upsert.set.field_path.drain(depth);
        }

        self.doc_mut().insert(upsert.set.field_path().path(false), new_if);

        Ok(())
    }

    pub(crate) fn insert_unset(&mut self, unset: operation::Unset) {
        let (target, _) = unset.field_path().path.split_last().unwrap();

        self.keys_to_unset.push(target.clone());
    }

    /// Given a mutable `Document` and a `FieldPath`, fills nested empty documents to the mutable `Document`.
    /// Returns a mutable reference to the most nested document along side the key to insert a value.
    /// ```text
    /// ["a", "b", "c"] -> ("c", doc! { "a": { "b": {} } })
    /// ```
    fn fill_empty_documents<'a, 'b>(&'a mut self, field_path: &'b FieldPath) -> (&'b String, &'a mut MergeObjects) {
        let (target, segments) = field_path.path.split_last().unwrap();

        let inner_merge = segments.iter().enumerate().fold(self, |acc, (i, segment)| {
            let mut merge_path = field_path.clone();
            merge_path.drain(i);

            let inner_merge = acc
                .doc_mut()
                .entry(segment.to_string())
                .or_insert(MergeObjects::new(merge_path).into())
                .as_merge_objects_mut()
                .unwrap();

            inner_merge
        });

        (target, inner_merge)
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

impl From<MergeDocument> for UpdateExpression {
    fn from(merge_doc: MergeDocument) -> Self {
        Self::MergeDocument(merge_doc)
    }
}
impl From<MergeObjects> for UpdateExpression {
    fn from(merge_objects: MergeObjects) -> Self {
        Self::MergeObjects(merge_objects)
    }
}
