use super::orderby::order_by_relation_prefix;
use crate::IntoBson;
use mongodb::bson::{doc, Bson, Document};
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
        // let cursor_fields: Vec<ScalarFieldRef> = cursor.fields().collect();
        // let cursor_values: Vec<Bson> = cursor
        //     .pairs
        //     .iter()
        //     .map(|(f, v)| (f, v.clone()).into_bson())
        //     .collect::<crate::Result<Vec<_>>>()?;

        // First, we need to pin the record specified by the cursor to find the order by values for the comparator.
        let mut cursor_filters = vec![];

        for (field, value) in cursor {
            let bson = (&field, value).into_bson()?;

            cursor_filters.push(doc! { field.db_name(): { "$eq": bson }});
        }

        let cursor_filter = doc! { "$and": cursor_filters };
        let mut bindings = Document::new();

        for (index, order_by) in self.order.iter().enumerate() {
            let prefix = order_by_relation_prefix(index, &order_by.path);

            let bind_field_name = if let Some(prefix) = prefix {
                prefix.first().unwrap().to_string()
            } else {
                order_by.field.db_name().to_owned()
            };

            // For: `"let": { fieldName: "$fieldName" }` bindings for the outer pipeline.
            bindings.insert(bind_field_name.clone(), format!("${}", bind_field_name));
        }

        // let cursor_condition = ;

        Ok(CursorData {
            cursor_filter,
            bindings,
            cursor_condition: todo!(),
        })
    }
}
