use crate::{FieldSelection, OrderBy};

/// Checks that the ordering is compatible with native DISTINCT ON in connectors that support it.
///
/// If order by is present, distinct fields must match leftmost order by fields in the query. The
/// order of the distinct fields does not necessarily have to be the same as the order of the
/// corresponding fields in the leftmost subset of `order_by` but all distinct fields must come
/// before non-distinct fields in the order by clause.
///
/// If there's no order by, then DISTINCT ON is allowed for any fields.
pub fn native_distinct_compatible_with_order_by(
    distinct_fields: Option<&FieldSelection>,
    order_by_fields: &[OrderBy],
) -> bool {
    if order_by_fields.is_empty() {
        return true;
    }

    let Some(distinct_fields) = distinct_fields else {
        return true;
    };

    let count_leftmost_matching = order_by_fields
        .iter()
        .take_while(|order_by| match order_by {
            OrderBy::Scalar(scalar) => distinct_fields.scalars().any(|sf| *sf == scalar.field),
            _ => false,
        })
        .count();

    count_leftmost_matching == distinct_fields.as_ref().len()
}
