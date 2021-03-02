use super::orderby::order_by_relation_prefix;
use crate::{orderby, IntoBson};
use mongodb::bson::{doc, Bson, Document};
use prisma_models::{OrderBy, RecordProjection, ScalarFieldRef, SortOrder};

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

        let cursor_condition = cursor_conditions(self.order, self.reverse);

        Ok(CursorData {
            cursor_filter,
            bindings,
            cursor_condition,
        })
    }
}

fn cursor_conditions(mut order_bys: Vec<OrderBy>, reverse: bool) -> Document {
    // let mut conditions = vec![];
    let num_orderings = order_bys.len();

    let doc = if num_orderings == 1 {
        let order_by = order_bys.pop().unwrap();
        map_orderby_condition(0, &order_by, reverse, true)
    } else {
        todo!()
    };

    doc
}

fn map_orderby_condition(index: usize, order_by: &OrderBy, reverse: bool, include_eq: bool) -> Document {
    let prefix = order_by_relation_prefix(index, &order_by.path);

    let order_field = if let Some(prefix) = prefix {
        format!("{}.{}", prefix.to_string(), order_by.field.db_name())
    } else {
        order_by.field.db_name().to_owned()
    };

    // let (field, field_alias) = &order_definition.field_aliased;
    // let cmp_column = Column::from((ORDER_TABLE_ALIAS, field_alias.to_owned()));
    // let cloned_cmp_column = cmp_column.clone();
    // let order_column = order_definition.order_column.clone();
    // let cloned_order_column = order_column.clone();

    let order_doc: Document = match order_by.sort_order {
        // If it's ASC but we want to take from the back, the ORDER BY will be DESC, meaning that comparisons done need to be lt(e).
        SortOrder::Ascending if reverse => {
            if include_eq {
                doc! { "$lte": [format!("${}", &order_field), format!("$${}", &order_field)] }
            } else {
                doc! { "$lt": [format!("${}", &order_field), format!("$${}", &order_field)] }
            }
        }

        // If it's DESC but we want to take from the back, the ORDER BY will be ASC, meaning that comparisons done need to be gt(e).
        SortOrder::Descending if reverse => {
            if include_eq {
                doc! { "$gte": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.greater_than_or_equals(cmp_column)
            } else {
                doc! { "$gt": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.greater_than(cmp_column)
            }
        }

        SortOrder::Ascending => {
            if include_eq {
                doc! { "$gte": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.greater_than_or_equals(cmp_column)
            } else {
                doc! { "$gt": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.greater_than(cmp_column)
            }
        }

        SortOrder::Descending => {
            if include_eq {
                doc! { "$lte": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.less_than_or_equals(cmp_column)
            } else {
                doc! { "$lt": [format!("${}", &order_field), format!("$${}", &order_field)] }
                // order_column.less_than(cmp_column)
            }
        }
    }
    .into();

    // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
    // statement over their order relative to the cursor.
    // let order_doc = if !order_by.field.is_required {

    //     // order_expr
    //     //     .or(cloned_order_column.is_null())
    //     //     .or(cloned_cmp_column.is_null())
    //     //     .into()
    // } else {
    //     order_expr
    // };

    // Add OR statements for the foreign key fields too if they are nullable
    // let order_expr = if let Some(fks) = &order_definition.fks {
    //     fks.iter()
    //         .filter(|(fk, _)| !fk.is_required)
    //         .fold(order_expr, |acc, (fk, alias)| {
    //             let col = if let Some(alias) = alias {
    //                 Column::from((alias.to_owned(), fk.db_name().to_owned()))
    //             } else {
    //                 fk.as_column()
    //             }
    //             .is_null();

    //             acc.or(col).into()
    //         })
    // } else {
    //     order_expr
    // };

    // order_expr

    doc! { "$expr": order_doc }
}

// fn map_equality_condition(field: &AliasedScalar, order_column: Column<'static>) -> Expression<'static> {
//     let (field, field_alias) = field;
//     let cmp_column = Column::from((ORDER_TABLE_ALIAS, field_alias.to_owned()));

//     // If we have null values in the ordering or comparison row, those are automatically included because we can't make a
//     // statement over their order relative to the cursor.
//     if !field.is_required {
//         order_column
//             .clone()
//             .equals(cmp_column.clone())
//             .or(cmp_column.is_null())
//             .or(order_column.is_null())
//             .into()
//     } else {
//         order_column.equals(cmp_column).into()
//     }
// }
