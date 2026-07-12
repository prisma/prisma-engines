use crate::{IntoBson, orderby::OrderByData};
use bson::{Document, doc};
use query_structure::{OrderBy, SelectionResult, SortOrder};

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
    cursor: SelectionResult,

    /// Ordering to use. Influences how cursor conditions are build.
    /// Relies on the `OrderByBuilder` to compute the joins and prepare
    /// the documents for cursor conditions so this builder can focus on
    /// conditionals only.
    order_data: Vec<OrderByData>,

    /// Order needs reversal
    reverse: bool,
}

impl CursorBuilder {
    pub fn new(cursor: SelectionResult, order: Vec<OrderBy>, reverse: bool) -> Self {
        let order_data = OrderByData::from_list(order);

        Self {
            cursor,
            order_data,
            reverse,
        }
    }

    /// Returns a filter document for the cursor and a filter document for the cursor condition.
    pub fn build(self) -> crate::Result<CursorData> {
        let cursor = self.cursor;

        // First, we need to pin the record specified by the cursor to find the order by values for the comparator.
        let mut cursor_filters = vec![];

        for (field, value) in cursor {
            let bson = (&field, value).into_bson()?;
            let field_name = format!("${}", field.db_name());

            cursor_filters.push(doc! { "$eq": [field_name, bson]});
        }

        let cursor_filter = doc! { "$and": cursor_filters };
        let mut bindings = Document::new();

        for order_data in self.order_data.iter() {
            let (left_bind_field_name, right_binding_field_name) = order_data.binding_names();

            // For: `"let": { fieldName: "$fieldName" }` bindings for the outer pipeline.
            bindings.insert(left_bind_field_name, format!("${right_binding_field_name}"));
        }

        let cursor_condition = cursor_conditions(self.order_data, self.reverse);

        Ok(CursorData {
            cursor_filter,
            bindings,
            cursor_condition,
        })
    }
}

fn cursor_conditions(mut order_data: Vec<OrderByData>, reverse: bool) -> Document {
    // let mut conditions = vec![];
    let num_orderings = order_data.len();

    if num_orderings == 1 {
        let order_data = order_data.pop().unwrap();
        map_orderby_condition(&order_data, reverse, true)
    } else {
        let mut or_conditions = vec![];

        for n in 0..num_orderings {
            let (head, tail) = order_data.split_at(num_orderings - n - 1);
            let mut and_conditions = Vec::with_capacity(head.len() + 1);

            for order_data in head {
                and_conditions.push(map_equality_condition(order_data));
            }

            let order_data = tail.first().unwrap();

            if head.len() == num_orderings - 1 {
                and_conditions.push(map_orderby_condition(order_data, reverse, true));
            } else {
                and_conditions.push(map_orderby_condition(order_data, reverse, false));
            }

            or_conditions.push(doc! { "$and": and_conditions });
        }

        doc! { "$or": or_conditions }
    }
}

fn map_orderby_condition(order_data: &OrderByData, reverse: bool, include_eq: bool) -> Document {
    let bound_order_field_reference = order_data.full_reference_path(true);
    let unbound_order_field_reference = order_data.full_reference_path(false);

    let order_doc: Document = match order_data.sort_order() {
        // If it's ASC but we want to take from the back, the ORDER BY will be DESC, meaning that comparisons done need to be lt(e).
        SortOrder::Ascending if reverse => {
            if include_eq {
                doc! { "$lte": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            } else {
                doc! { "$lt": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            }
        }

        // If it's DESC but we want to take from the back, the ORDER BY will be ASC, meaning that comparisons done need to be gt(e).
        SortOrder::Descending if reverse => {
            if include_eq {
                doc! { "$gte": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            } else {
                doc! { "$gt": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            }
        }

        SortOrder::Ascending => {
            if include_eq {
                doc! { "$gte": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            } else {
                doc! { "$gt": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            }
        }

        SortOrder::Descending => {
            if include_eq {
                doc! { "$lte": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            } else {
                doc! { "$lt": [format!("${}", &unbound_order_field_reference), format!("$${}", &bound_order_field_reference)] }
            }
        }
    };

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

    order_doc
}

fn map_equality_condition(order_data: &OrderByData) -> Document {
    let order_field_reference = order_data.full_reference_path(true);

    // // If we have null values in the ordering or comparison row, those are automatically
    // // included because we can't make a statement over their order relative to the cursor.
    // if !field.is_required {
    //     order_column
    //         .clone()
    //         .equals(cmp_column.clone())
    //         .or(cmp_column.is_null())
    //         .or(order_column.is_null())
    //         .into()

    //     doc! {
    //         "$or": [
    //             { "$expr": { "$eq": [format!("${}", &order_field_reference), format!("$${}", &order_field_reference)] }},
    //             { "$expr": { "$eq": [format!("${}", &order_field_reference), format!("$${}", &order_field_reference)] }},
    //             { "$expr": { "$eq": [format!("${}", &order_field_reference), format!("$${}", &order_field_reference)] }}
    //         ]
    //     }
    // } else {

    // Todo: Verify this actually does everything we want already.
    doc! { "$eq": [format!("${}", &order_field_reference), format!("$${}", &order_field_reference)] }
    // }
}
