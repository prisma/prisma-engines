use super::*;
use schema::constants::aggregations::*;

pub(crate) fn extract_nested_rel_aggr_selections(
    field_pairs: Vec<FieldPair<'_>>,
) -> (Vec<FieldPair<'_>>, Vec<FieldPair<'_>>) {
    field_pairs.into_iter().partition(is_aggr_selection)
}

pub(crate) fn is_aggr_selection(pair: &FieldPair<'_>) -> bool {
    matches!(pair.parsed_field.name.as_str(), UNDERSCORE_COUNT)
}
