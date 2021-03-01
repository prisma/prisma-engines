use crate::IntoBson;
use mongodb::bson::{Bson, Document};
use prisma_models::{OrderBy, RecordProjection, ScalarFieldRef};

#[derive(Debug, Clone)]
pub(crate) struct CursorData {
    /// A mongo filter document containing cursor-related conditions.
    /// These are applied after sort operations but before skip or takes.
    pub cursor_filter: Document,

    /// A document to make the `let` binding of the outer (cursor) document work.
    pub bindings: Document,

    /// The actual conditions used to filter down records based on the cursor.
    pub cursor_condition: Document,
}

#[derive(Debug, Default)]
pub(crate) struct CursorBuilder {
    /// The field-value combination the cursor .
    cursor: RecordProjection,

    /// Ordering to use. Influences how cursor conditions are build.
    /// Relies on the `OrderByBuilder` to compute the joins and prepare
    /// the documents for cursor conditions so this builder can focus on
    /// conditionals only.
    order: Vec<OrderBy>,

    /// Order needs reversal
    reverse: bool,
}

impl CursorBuilder {
    pub fn new(cursor: RecordProjection, order: Vec<OrderBy>, reverse: bool) -> Self {
        Self { cursor, order, reverse }
    }

    /// Returns a filter document for the cursor and a filter document for the cursor condition.
    pub fn build(self) -> crate::Result<CursorData> {
        let cursor = self.cursor;
        let cursor_fields: Vec<ScalarFieldRef> = cursor.fields().collect();
        let cursor_values: Vec<Bson> = cursor
            .pairs
            .iter()
            .map(|(f, v)| (f, v.clone()).into_bson())
            .collect::<crate::Result<Vec<_>>>()?;

        // First, we need to pin the record specified by the cursor to find the order by values for the comparator.

        // Returns filter document
        todo!()
    }
}
